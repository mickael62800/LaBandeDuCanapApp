import { computed, ref } from "vue";
import { useToast } from "./useToast";
import {
  nexusGamesService,
  type ServerSchedule,
} from "@/services/nexusGamesService";

/**
 * Plages d'ouverture automatique d'un serveur de jeu.
 *
 * Extraites de la page de détail pour rester testables seules : la logique
 * (conversion minutes ↔ « HH:MM », garde d'état du formulaire, état de
 * sauvegarde) est indépendante de Vue, hormis les `ref` exposés.
 *
 * Les heures sont saisies en heure LOCALE : c'est ce que lit un administrateur,
 * et le fuseau enregistré avec évite le décalage d'une heure aux changements
 * de saison.
 */
export function useGameServerSchedule(
  guildId: () => string | undefined,
  serverId: () => string | undefined,
) {
  const { success, error: showError } = useToast();

  const enabled = ref(false);
  const timezone = ref("Europe/Paris");
  /// Préavis avant fermeture, en minutes.
  const warn = ref(10);
  /// Plages en « HH:MM », plus lisibles à l'écran que des minutes depuis minuit.
  const ranges = ref<{ start: string; end: string }[]>([]);
  /// Prochaine ouverture, calculée par le serveur.
  const nextOpening = ref<string | null>(null);
  /// Réglages de redémarrage automatique neutralisés par les plages.
  const disabledRestartKeys = ref<string[]>([]);
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
      enabled.value = data.enabled;
      timezone.value = data.timezone;
      warn.value = data.warn_minutes;
      ranges.value = data.ranges.map((r) => ({
        start: minutesVersHeure(r.start_minute),
        end: minutesVersHeure(r.end_minute),
      }));
      nextOpening.value = data.next_opening;
      disabledRestartKeys.value = data.disabled_restart_keys;
    } catch {
      // Horaires indisponibles : on garde le formulaire tel quel plutôt que de
      // le vider sous les yeux de l'administrateur.
    }
  }

  function ajouterPlage() {
    ranges.value.push({ start: "19:00", end: "23:00" });
  }

  function retirerPlage(index: number) {
    ranges.value.splice(index, 1);
  }

  async function save() {
    const g = guildId();
    const s = serverId();
    if (!g || !s || saving.value) return;
    saving.value = true;
    try {
      const resultat = await nexusGamesService.saveScheduleRanges(g, s, {
        enabled: enabled.value,
        timezone: timezone.value,
        warn_minutes: warn.value,
        ranges: ranges.value.map((r) => ({
          start_minute: heureVersMinutes(r.start),
          end_minute: heureVersMinutes(r.end),
        })),
      });
      // Le serveur renvoie l'état recalculé : on le prend tel quel, au lieu de
      // supposer que ce qu'on a envoyé est ce qu'il a retenu.
      nextOpening.value = resultat.next_opening;
      disabledRestartKeys.value = resultat.disabled_restart_keys;
      success("Horaires enregistrés.");
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

  return {
    enabled,
    timezone,
    warn,
    ranges,
    nextOpening,
    disabledRestartKeys,
    saving,
    prochaineOuverture,
    load,
    ajouterPlage,
    retirerPlage,
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
  guildId: () => string | undefined,
  serverId: () => string | undefined,
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
