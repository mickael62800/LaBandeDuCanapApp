import { computed, ref } from "vue";
import { useToast } from "./useToast";
import {
  nexusGamesService,
  type ScheduleMode,
  type ServerSchedule,
} from "@/services/nexusGamesService";

/**
 * Les sept jours, dans l'ordre du masque envoyé par l'API : lundi porte le
 * bit 0, dimanche le bit 6.
 *
 * L'ordre suit celui du domaine Rust (`num_days_from_monday`). Le changer ici
 * décalerait tous les horaires d'un jour sans qu'aucun test d'affichage ne
 * s'en aperçoive — c'est pourquoi il n'est pas dérivé d'une locale.
 */
export const JOURS = [
  { bit: 1, court: "Lun", long: "lundi" },
  { bit: 2, court: "Mar", long: "mardi" },
  { bit: 4, court: "Mer", long: "mercredi" },
  { bit: 8, court: "Jeu", long: "jeudi" },
  { bit: 16, court: "Ven", long: "vendredi" },
  { bit: 32, court: "Sam", long: "samedi" },
  { bit: 64, court: "Dim", long: "dimanche" },
] as const;

/** Les sept bits réunis. */
export const TOUS_LES_JOURS = 0b111_1111;

/** Ce jour est-il coché dans ce masque ? */
export function jourActif(masque: number, bit: number): boolean {
  return (masque & bit) !== 0;
}

/**
 * Pilotage d'un serveur de jeu dans le temps.
 *
 * Deux systèmes, et un seul à la fois :
 *
 *   - **plages d'ouverture** : le serveur ne tourne que sur les créneaux
 *     déclarés (« 18h-20h ») ;
 *   - **permanence** : il tourne en continu et redémarre à intervalle régulier,
 *     parce qu'un jeu qui tourne des jours d'affilée ne rend pas la mémoire
 *     qu'il prend.
 *
 * L'exclusion vit côté serveur (une seule colonne `mode`) : cet écran ne fait
 * que la refléter. Il n'a aucune autorité sur elle — c'est l'API qui tranche.
 *
 * Extrait de la page de détail pour rester testable seul : la logique
 * (conversion minutes ↔ « HH:MM », garde d'état du formulaire, état de
 * sauvegarde) est indépendante de Vue, hormis les `ref` exposés.
 *
 * Les heures sont saisies en heure LOCALE : c'est ce que lit un administrateur,
 * et le fuseau enregistré avec évite le décalage d'une heure aux changements
 * de saison.
 */
export function useGameServerSchedule(
  guildId: () => string | null | undefined,
  serverId: () => string | null | undefined,
) {
  const { success, error: showError } = useToast();

  const enabled = ref(false);
  const mode = ref<ScheduleMode>("ranges");
  const timezone = ref("Europe/Paris");
  /// Préavis avant fermeture ou redémarrage, en minutes.
  const warn = ref(10);
  /// Plages en « HH:MM », plus lisibles à l'écran que des minutes depuis minuit.
  const ranges = ref<{ start: string; end: string; days: number }[]>([]);
  /// Prochaine ouverture, calculée par le serveur.
  const nextOpening = ref<string | null>(null);
  /// Réglages de redémarrage automatique neutralisés par les plages.
  const disabledRestartKeys = ref<string[]>([]);
  /// Mode permanence : heures entre deux redémarrages.
  const restartIntervalHours = ref<number | null>(null);
  const restartAnchorMinute = ref(0);
  const nextRestart = ref<string | null>(null);
  /// Cadences proposées, dictées par le serveur.
  const restartIntervalChoices = ref<number[]>([]);
  const saving = ref(false);

  function minutesVersHeure(minutes: number): string {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}`;
  }

  function heureVersMinutes(valeur: string): number {
    const [h, m] = valeur.split(":").map(Number);
    return (h || 0) * 60 + (m || 0);
  }

  async function load() {
    const g = guildId();
    const s = serverId();
    if (!g || !s) return;
    try {
      const data: ServerSchedule = await nexusGamesService.getScheduleRanges(g, s);
      appliquer(data);
      ranges.value = data.ranges.map((r) => ({
        start: minutesVersHeure(r.start_minute),
        end: minutesVersHeure(r.end_minute),
        days: r.days ?? TOUS_LES_JOURS,
      }));
    } catch {
      // Horaires indisponibles : on garde le formulaire tel quel plutôt que de
      // le vider sous les yeux de l'administrateur.
    }
  }

  /**
   * Recopie l'état renvoyé par le serveur.
   *
   * On le prend tel quel plutôt que de supposer que ce qu'on a envoyé est ce
   * qu'il a retenu : il borne le préavis, refuse une cadence hors liste, et
   * recalcule les prochaines échéances.
   */
  function appliquer(data: ServerSchedule) {
    enabled.value = data.enabled;
    mode.value = data.mode;
    timezone.value = data.timezone;
    warn.value = data.warn_minutes;
    nextOpening.value = data.next_opening;
    nextRestart.value = data.next_restart;
    disabledRestartKeys.value = data.disabled_restart_keys;
    restartIntervalHours.value = data.restart_interval_hours;
    restartAnchorMinute.value = data.restart_anchor_minute;
    restartIntervalChoices.value = data.restart_interval_choices;
  }

  function ajouterPlage() {
    ranges.value.push({ start: "19:00", end: "23:00", days: TOUS_LES_JOURS });
  }

  function retirerPlage(index: number) {
    ranges.value.splice(index, 1);
  }

  /** Coche ou décoche un jour pour une plage donnée. */
  function basculerJour(index: number, bit: number) {
    const plage = ranges.value[index];
    if (!plage) return;
    plage.days = jourActif(plage.days, bit) ? plage.days & ~bit : plage.days | bit;
  }

  /**
   * Étend une plage à toute la semaine.
   *
   * Le cas courant est « les mêmes horaires tous les jours » : cocher sept
   * cases à la main pour chaque plage est fastidieux et se prête aux oublis —
   * un jour manquant ne se voit pas, le serveur reste simplement éteint ce
   * jour-là sans que rien ne l'explique.
   */
  function appliquerATousLesJours(index: number) {
    const plage = ranges.value[index];
    if (!plage) return;
    plage.days = TOUS_LES_JOURS;
  }

  /**
   * Bascule vers la permanence en proposant une cadence par défaut.
   *
   * Sans elle, activer la permanence serait refusé par l'API faute de cadence,
   * et l'administrateur verrait une erreur là où il attend un réglage.
   */
  function choisirMode(nouveau: ScheduleMode) {
    mode.value = nouveau;
    if (nouveau === "restart" && restartIntervalHours.value === null) {
      restartIntervalHours.value = 6;
      // Un quart d'heure laisse le temps de finir ce qu'on fait ; c'est le
      // préavis qui a du sens pour un redémarrage, pas les 10 min d'une
      // fermeture de soirée.
      warn.value = 15;
    }
  }

  async function save() {
    const g = guildId();
    const s = serverId();
    if (!g || !s || saving.value) return;
    saving.value = true;
    try {
      const resultat = await nexusGamesService.saveScheduleRanges(g, s, {
        enabled: enabled.value,
        mode: mode.value,
        timezone: timezone.value,
        warn_minutes: warn.value,
        restart_interval_hours: restartIntervalHours.value,
        restart_anchor_minute: restartAnchorMinute.value,
        ranges: ranges.value.map((r) => ({
          start_minute: heureVersMinutes(r.start),
          end_minute: heureVersMinutes(r.end),
          days: r.days,
        })),
      });
      appliquer(resultat);
      success(
        resultat.mode === "restart"
          ? "Permanence enregistrée."
          : "Horaires enregistrés.",
      );
    } catch (e) {
      showError(e instanceof Error ? e.message : "Enregistrement impossible");
    } finally {
      saving.value = false;
    }
  }

  const prochaineOuverture = computed(() =>
    nextOpening.value
      ? new Date(nextOpening.value).toLocaleString("fr-FR")
      : null,
  );

  const prochainRedemarrage = computed(() =>
    nextRestart.value
      ? new Date(nextRestart.value).toLocaleString("fr-FR")
      : null,
  );

  const estPermanence = computed(() => mode.value === "restart");

  return {
    enabled,
    mode,
    timezone,
    warn,
    ranges,
    nextOpening,
    nextRestart,
    disabledRestartKeys,
    restartIntervalHours,
    restartAnchorMinute,
    restartIntervalChoices,
    saving,
    prochaineOuverture,
    prochainRedemarrage,
    estPermanence,
    load,
    ajouterPlage,
    retirerPlage,
    basculerJour,
    appliquerATousLesJours,
    choisirMode,
    save,
  };
}

/**
 * Alertes de supervision d'un serveur de jeu (seuils + webhook Discord).
 *
 * Elles vivaient dans le navigateur : seuils et webhook en `localStorage`,
 * vérification à chaque rafraîchissement de la page. Fermer l'onglet arrêtait
 * donc la surveillance — or c'est la nuit, page fermée, qu'un serveur sature.
 * Tout est passé côté serveur.
 *
 * L'URL du webhook est un secret : elle ne revient jamais au front, l'écran
 * sait seulement qu'un webhook est enregistré.
 */
export function useGameServerAlerts(
  guildId: () => string | null | undefined,
  serverId: () => string | null | undefined,
) {
  const { success, error: showError } = useToast();

  const cpuThreshold = ref(85);
  const ramThreshold = ref(90);
  /// Seuil de temps de réponse : la mesure qui correspond au lag ressenti.
  /// CPU et RAM disent ce que le conteneur consomme, celle-ci ce que les
  /// joueurs subissent — un serveur peut ramer à 30 % de processeur.
  const latencyThreshold = ref(500);
  const webhookUrl = ref("");
  const configured = ref(false);
  const saving = ref(false);

  async function load() {
    const g = guildId();
    const s = serverId();
    if (!g || !s) return;
    try {
      const settings = await nexusGamesService.getAlertSettings(g, s);
      configured.value = settings.configured;
      cpuThreshold.value = settings.cpu_threshold;
      ramThreshold.value = settings.ram_threshold;
      latencyThreshold.value = settings.latency_threshold_ms;
    } catch {
      // Réglages indisponibles : on garde les valeurs par défaut affichées
      // plutôt que de vider le formulaire sous les yeux de l'administrateur.
    }
  }

  async function save() {
    const g = guildId();
    const s = serverId();
    if (!g || !s || saving.value) return;
    saving.value = true;
    try {
      await nexusGamesService.saveAlertSettings(g, s, {
        // Champ laissé vide = on garde le webhook déjà enregistré.
        webhook_url: webhookUrl.value.trim() || undefined,
        cpu_threshold: cpuThreshold.value,
        ram_threshold: ramThreshold.value,
        latency_threshold_ms: latencyThreshold.value,
      });
      webhookUrl.value = "";
      success("Alertes enregistrées. La surveillance tourne côté serveur, page fermée comprise.");
      await load();
    } catch (e) {
      showError(e instanceof Error ? e.message : "Enregistrement impossible");
    } finally {
      saving.value = false;
    }
  }

  async function disable() {
    const g = guildId();
    const s = serverId();
    if (!g || !s) return;
    if (!confirm("Arrêter la surveillance de ce serveur ? Le webhook enregistré sera supprimé.")) {
      return;
    }
    try {
      await nexusGamesService.deleteAlertSettings(g, s);
      success("Surveillance arrêtée.");
      await load();
    } catch (e) {
      showError(e instanceof Error ? e.message : "Arrêt impossible");
    }
  }

  return {
    cpuThreshold,
    ramThreshold,
    latencyThreshold,
    webhookUrl,
    configured,
    saving,
    load,
    save,
    disable,
  };
}
