<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import AppToggle from "../atoms/AppToggle.vue";
// Détail d'un serveur de jeu : pilotage, ressources, configuration, logs,
// console RCON et historique des joueurs.
//
// Choix structurants :
//   - les statistiques ne sont rafraîchies que si le serveur tourne. Interroger
//     Docker toutes les 5 s pour un conteneur arrêté ne renverrait que des
//     zéros, en payant une requête à chaque fois.
//   - la configuration éditable est générée depuis le `config_schema` du
//     template, comme le formulaire de création : un nouveau réglage ajouté en
//     base apparaît ici sans toucher au front.
//   - la console RCON n'apparaît que si le jeu la supporte ET que le serveur
//     tourne : afficher un champ qui échouera à coup sûr n'aide personne.

import { computed, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import { communityAdminService } from "../../services/communityAdminService";
import {
  nexusGamesService,
  adresseServeur,
  type GameServer,
  type GameTemplate,
  type PlayerSession,
} from "@/services/nexusGamesService";
import { useTemplateFieldGroups } from "@/composables/useTemplateFieldGroups";
import { useGameServerMonitoring, volume, debit } from "@/composables/useGameServerMonitoring";
import {
  useGameServerSchedule,
  useGameServerAlerts,
  JOURS,
  jourActif,
} from "@/composables/useGameServerSchedule";
import GameConfigField from "../molecules/GameConfigField.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import GameCommandPanel from "../organisms/GameCommandPanel.vue";

import { Line } from 'vue-chartjs'

const route = useRoute();
const router = useRouter();
const { selectedGuildId } = useGuildSelector();
// `useAuth` n'est plus consulte ici : l'auteur des actions vient desormais de
// la passerelle (en-tete `X-Actor-Id`), plus du navigateur.
const { success, error: showError } = useToast();

const serverId = computed(() => String(route.params.id ?? ""));

const server = ref<GameServer | null>(null);
const config = ref<Record<string, string>>({});
const template = ref<GameTemplate | null>(null);
const logs = ref<string[]>([]);
const sessions = ref<PlayerSession[]>([]);
// L'historique des joueurs grossit a chaque connexion et n'est jamais purge :
// l'onglet demandait tout et n'en affichait qu'un debut arbitraire — les cent
// dernieres sessions, en laissant croire que c'etait l'historique entier.
//
// La pagination se fait cote SERVEUR (`limit` / `offset`, deja offerts par
// l'API) et non avec `usePagination`, qui decoupe un tableau deja charge :
// telecharger des milliers de lignes pour en montrer vingt-cinq deplacerait
// simplement le probleme. Seule la barre est partagee avec les autres listes,
// pour que la commande se manipule pareil partout.
const sessionsPage = ref(1);
const sessionsParPage = ref(25);
const sessionsTotal = ref(0);
const sessionsPages = computed(() =>
  Math.max(1, Math.ceil(sessionsTotal.value / sessionsParPage.value)),
);
/// La fermeture planifiee vit dans l'evenement communautaire associe au
/// serveur (Nexus ne stocke pour l'instant que l'heure d'ouverture).
const plannedStopAt = ref<string | null>(null);

const loading = ref(false);
const errorMessage = ref("");
const busy = ref(false);
const savingConfig = ref(false);
const revealingIp = ref(false);
const showScheduleForm = ref(false);
const scheduling = ref(false);
/// Valeur du champ `datetime-local` (heure locale « YYYY-MM-DDTHH:mm »).
const revealAtInput = ref("");
const stopAtInput = ref("");
const rconCommand = ref("");
const rconOutput = ref("");

type Onglet =
  | "apercu"
  | "config"
  | "surveillance"
  | "logs"
  | "commandes"
  | "console"
  | "joueurs";
const onglet = ref<Onglet>("apercu");

const isRunning = computed(() => server.value?.status === "running");

// ── Surveillance : stats en direct + courbes d'historique (délégué au composable) ──
const {
  stats,
  historiqueEnCours,
  PLAGES,
  plageChoisie,
  pasLisible,
  changerPlage,
  loadHistorique,
  chartOptions,
  chartOptionsAuto,
  chartOptionsReseau,
  chartOptionsJoueurs,
  cpuChartData,
  ramChartData,
  netChartData,
  latencyChartData,
  playersChartData,
} = useGameServerMonitoring(
  () => selectedGuildId.value,
  () => serverId.value,
  isRunning,
);

// ── Plages d'ouverture automatiques (délégué au composable) ──
const {
  enabled: scheduleEnabled,
  timezone: scheduleTimezone,
  warn: scheduleWarn,
  ranges: scheduleRanges,
  disabledRestartKeys: scheduleDisabledKeys,
  restartIntervalHours,
  restartAnchorMinute,
  restartIntervalChoices,
  estPermanence,
  saving: savingSchedule,
  save: saveSchedule,
  load: loadSchedule,
  prochaineOuverture,
  prochainRedemarrage,
  ajouterPlage,
  retirerPlage,
  basculerJour,
  appliquerATousLesJours,
  choisirMode,
} = useGameServerSchedule(
  () => selectedGuildId.value,
  () => serverId.value,
);

// ── Alertes de supervision (délégué au composable) ──
const {
  cpuThreshold,
  ramThreshold,
  latencyThreshold,
  webhookUrl,
  configured: alertsConfigured,
  saving: savingAlerts,
  save: saveAlertSettings,
  disable: disableAlerts,
  load: loadAlerts,
} = useGameServerAlerts(
  () => selectedGuildId.value,
  () => serverId.value,
);

/// Adresse de connexion, disponible des la creation cote administration.
const adresse = computed(() => (server.value ? adresseServeur(server.value) : null));

/// `writeText` echoue hors HTTPS ou sans autorisation : on le dit plutot que
/// de laisser croire au succes.
async function copier(valeur: string) {
  try {
    await navigator.clipboard.writeText(valeur);
    success(`Adresse copiee : ${valeur}`);
  } catch {
    showError("Copie impossible, selectionne l'adresse a la main");
  }
}

const isTransient = computed(
  () => server.value?.status === "starting" || server.value?.status === "stopping",
);

let transientTimer: ReturnType<typeof setInterval> | null = null;
watch(isTransient, (transient) => {
  if (transientTimer) {
    clearInterval(transientTimer);
    transientTimer = null;
  }
  if (transient) {
    transientTimer = setInterval(load, 2000);
  }
}, { immediate: true });
onUnmounted(() => transientTimer && clearInterval(transientTimer));

const isScheduled = computed(() => server.value?.status === "scheduled");

const STATUS_LABELS: Record<string, string> = {
  created: "Créé",
  scheduled: "En attente d'ouverture",
  starting: "Démarrage…",
  running: "En ligne",
  stopping: "Arrêt…",
  stopped: "Arrêté",
  error: "Erreur",
  deleted: "Supprimé",
};

async function load() {
  if (!selectedGuildId.value || !serverId.value) return;
  loading.value = true;
  errorMessage.value = "";
  try {
    const detail = await nexusGamesService.getServer(selectedGuildId.value, serverId.value);
    server.value = detail.server;
    config.value = { ...detail.config };

    // Retrouver l'evenement cree avec le serveur. Plusieurs anciens formats de
    // titre existent, mais ils contiennent tous le nom exact du serveur. Si
    // plusieurs correspondent, prendre celui dont le debut est le plus proche
    // de l'ouverture Nexus evite d'afficher une ancienne session homonyme.
    plannedStopAt.value = null;
    const openingMs = detail.server.ip_reveal_at
      ? new Date(detail.server.ip_reveal_at).getTime()
      : new Date(detail.server.created_at).getTime();
    const events = await communityAdminService
      .listEvents(
        selectedGuildId.value,
        new Date(openingMs - 90 * 86400 * 1000),
        new Date(openingMs + 180 * 86400 * 1000),
      )
      .catch(() => []);
    const matchingEvent = events
      .filter((event) => event.title.toLowerCase().includes(detail.server.name.toLowerCase()))
      .sort(
        (a, b) =>
          Math.abs(new Date(a.starts_at).getTime() - openingMs)
          - Math.abs(new Date(b.starts_at).getTime() - openingMs),
      )[0];
    plannedStopAt.value = matchingEvent?.ends_at ?? null;

    // Le template porte le schéma des réglages et le support RCON.
    const list = await nexusGamesService
      .listTemplates(selectedGuildId.value)
      .catch(() => [] as GameTemplate[]);
    template.value = list.find((t) => t.id === detail.server.template_id) ?? null;
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    server.value = null;
  } finally {
    loading.value = false;
  }
}

async function act(action: "start" | "stop" | "restart") {
  if (!selectedGuildId.value || !server.value || busy.value) return;
  busy.value = true;
  try {
    await nexusGamesService[action](selectedGuildId.value, server.value.id);
    success("Action envoyée.");
    // Le conteneur met quelques secondes à changer d'état : on laisse Docker
    // faire avant de relire, sinon on réafficherait l'état précédent.
    setTimeout(load, 1500);
  } catch (e) {
    showError(e instanceof Error ? e.message : "Action impossible");
  } finally {
    busy.value = false;
  }
}

async function remove() {
  if (!selectedGuildId.value || !server.value) return;
  if (!confirm(`Supprimer définitivement « ${server.value.name} » et ses données ?`)) return;
  const guildId = selectedGuildId.value;
  const serverId = server.value.id;
  try {
    await nexusGamesService.remove(guildId, serverId);
    await retirerLaSoireeDuCalendrier(guildId, serverId);
    success("Serveur supprimé.");
    router.push("/nexus/servers");
  } catch (e) {
    showError(e instanceof Error ? e.message : "Suppression impossible");
  }
}

/**
 * Retire du calendrier communautaire la soirée créée avec ce serveur.
 *
 * Créer un serveur inscrit une soirée au planning. Sans ce nettoyage, elle y
 * restait après la suppression : une session Terraria effacée le 21 août
 * s'annonçait encore « jusqu'au 21 septembre » sur le site public, et rien ne
 * permettait de repérer l'orphelin autrement qu'en lisant les titres.
 *
 * Le rapprochement se fait sur `source_server_id`, posé à la création. Aucune
 * clé étrangère n'est possible : `community_events` vit dans la base Sentinel,
 * `game_servers` dans celle de Nexus.
 *
 * N'interrompt jamais la suppression : le serveur est déjà parti à ce stade, et
 * échouer ici laisserait l'utilisateur croire que rien n'a eu lieu. Une soirée
 * qui survit se retire à la main ; un serveur à moitié supprimé, non.
 */
async function retirerLaSoireeDuCalendrier(guildId: string, serverId: string) {
  try {
    const evenements = await communityAdminService.listEvents(guildId);
    const lies = evenements.filter((e) => e.source_server_id === serverId);
    for (const evenement of lies) {
      await communityAdminService.deleteEvent(evenement.id);
    }
  } catch {
    // Volontairement muet vis-à-vis de l'utilisateur : voir ci-dessus.
  }
}

async function revealIpNow() {
  if (!selectedGuildId.value || !server.value || revealingIp.value) return;
  if (!confirm(`Révéler immédiatement l'adresse de « ${server.value.name} » à tous les membres ? Le rôle du jeu sera mentionné s'il existe.`)) return;
  revealingIp.value = true;
  try {
    await nexusGamesService.revealIp(selectedGuildId.value, server.value.id);
    success("Adresse révélée immédiatement.");
    await load();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Révélation impossible");
  } finally {
    revealingIp.value = false;
  }
}

async function startAndRevealIpNow() {
  if (!selectedGuildId.value || !server.value || busy.value || revealingIp.value) return;
  if (!confirm(`Démarrer « ${server.value.name} » et révéler l'adresse IP dès que le port sera alloué ?`)) return;
  
  busy.value = true;
  const guildId = selectedGuildId.value;
  const srvId = server.value.id;
  
  try {
    // 1. Démarre le serveur
    await nexusGamesService.start(guildId, srvId);
    success("Démarrage en cours... L'adresse sera révélée dès que le serveur sera en ligne.");
    
    // 2. Poll l'état jusqu'à ce qu'il soit 'running'
    let attempts = 0;
    while (attempts < 60) { // Timeout de 2 min max (60 * 2s)
      await new Promise(r => setTimeout(r, 2000));
      await load();
      if (!server.value) break;
      if (server.value.status === 'running') {
        // 3. Révèle l'IP sur Discord une fois le port alloué !
        await nexusGamesService.revealIp(guildId, srvId);
        success("Serveur démarré et IP révélée avec succès !");
        break;
      } else if (server.value.status === 'error' || server.value.status === 'stopped') {
        showError("Le serveur n'a pas pu démarrer.");
        break;
      }
      attempts++;
    }
    
    await load();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Action impossible");
  } finally {
    busy.value = false;
  }
}
/// Programme l'ouverture. Sur un serveur au repos -> mode « Préparation »
/// (le conteneur démarrera ~5 min avant, l'IP sera révélée à l'heure). Sur un
/// serveur déjà en ligne -> programme seulement la révélation auto de l'IP.
async function submitSchedule() {
  if (!selectedGuildId.value || !server.value || scheduling.value) return;
  if (!revealAtInput.value) return;
  const iso = new Date(revealAtInput.value).toISOString();
  if (new Date(iso).getTime() <= Date.now()) {
    showError("Choisis une date et une heure dans le futur.");
    return;
  }
  scheduling.value = true;
  try {
    if (isRunning.value) {
      await nexusGamesService.setRevealSchedule(
        selectedGuildId.value,
        server.value.id,
        iso,
      );
      success("Révélation de l'adresse programmée.");
    } else {
      await nexusGamesService.schedule(
        selectedGuildId.value,
        server.value.id,
        iso,
        stopAtInput.value ? new Date(stopAtInput.value).toISOString() : null,
      );
      success(isScheduled.value ? "Ouverture reprogrammée !" : "Ouverture programmée : les inscriptions sont ouvertes.");
    }

    // Création / synchronisation automatique sans doublon de l'événement dans le Planning Communautaire
    const endIso = stopAtInput.value
      ? new Date(stopAtInput.value).toISOString()
      : new Date(new Date(iso).getTime() + 4 * 3600 * 1000).toISOString(); // Par défaut +4h si pas de date de fermeture

    const eventTitle = `🎮 ${server.value.name}`;

    try {
      const windowFrom = new Date(Date.now() - 90 * 86400 * 1000);
      const windowTo = new Date(Date.now() + 180 * 86400 * 1000);
      const existingEvents = await communityAdminService.listEvents(selectedGuildId.value, windowFrom, windowTo);
      // Trouve tout événement dont le titre contient le nom du serveur
      const matches = existingEvents.filter((e) =>
        e.title === eventTitle ||
        e.title.includes(server.value!.name) ||
        (server.value!.name && e.title.toLowerCase().includes(server.value!.name.toLowerCase())),
      );

      const payload = {
        title: eventTitle,
        description: `Ouverture du serveur de jeu ${server.value.name}. Rejoignez-nous !`,
        game: template.value?.name ?? server.value.name,
        starts_at: iso,
        ends_at: endIso,
        is_public: true,
      };

      if (matches.length > 0) {
        // Met à jour le premier événement trouvé
        const firstMatch = matches[0];
        await communityAdminService.updateEvent(firstMatch.id, payload);

        // Supprime automatiquement tous les autres événements en doublon créés précédemment
        for (let i = 1; i < matches.length; i++) {
          await communityAdminService.deleteEvent(matches[i].id).catch(() => null);
        }

        success("Événement du Planning mis à jour (doublons nettoyés) !");
      } else {
        await communityAdminService.createEvent(selectedGuildId.value, payload);
        success("Événement inscrit au Planning Communautaire !");
      }
    } catch (e) {
      console.warn("Événement planning non synchronisé:", e);
    }

    showScheduleForm.value = false;
    await load();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Programmation impossible");
  } finally {
    scheduling.value = false;
  }
}

async function saveConfig() {
  if (!selectedGuildId.value || !server.value) return;
  savingConfig.value = true;
  try {
    // N'envoyer que les clés que le jeu déclare encore.
    //
    // L'API refuse toute clé hors schéma — protection voulue, sans quoi
    // n'importe quelle variable d'environnement pourrait être injectée dans le
    // conteneur. Mais un réglage retiré du schéma laisse sa valeur en base :
    // elle partait avec le reste et faisait échouer TOUT l'enregistrement, sur
    // un réglage que l'écran n'affiche même plus.
    const connues = new Set((template.value?.config_schema ?? []).map((f) => f.key));
    const aEnvoyer = Object.fromEntries(
      Object.entries(config.value).filter(([cle]) => connues.has(cle)),
    );

    await nexusGamesService.updateConfig(
      selectedGuildId.value,
      server.value.id,
      aEnvoyer,
    );
    success("Configuration enregistrée. Redémarre le serveur pour l'appliquer.");
  } catch (e) {
    showError(e instanceof Error ? e.message : "Enregistrement impossible");
  } finally {
    savingConfig.value = false;
  }
}

async function loadLogs() {
  if (!selectedGuildId.value || !server.value) return;
  try {
    logs.value = await nexusGamesService.logs(selectedGuildId.value, server.value.id, 300);
  } catch (e) {
    logs.value = [e instanceof Error ? e.message : "Logs indisponibles"];
  }
}

async function loadSessions() {
  if (!selectedGuildId.value || !server.value) return;
  const page = await nexusGamesService
    .sessions(selectedGuildId.value, server.value.id, {
      limit: sessionsParPage.value,
      offset: (sessionsPage.value - 1) * sessionsParPage.value,
    })
    .catch(() => null);
  // En cas d'echec, on garde la page affichee : la remplacer par du vide
  // ferait croire a un historique efface.
  if (!page) return;
  sessions.value = page.items;
  sessionsTotal.value = page.total;
}

// Changer de page ou de taille recharge depuis l'API. Le retour a la premiere
// page lors d'un changement de taille evite de demander un decalage qui
// n'existe plus — vingt-cinq par page, page 12, puis cent par page, et l'ecran
// resterait vide sans rien expliquer.
function allerALaPageDeSessions(page: number) {
  if (page < 1 || page > sessionsPages.value) return;
  sessionsPage.value = page;
  void loadSessions();
}

function changerTailleDeSessions(taille: number) {
  sessionsParPage.value = taille;
  sessionsPage.value = 1;
  void loadSessions();
}

async function sendRcon() {
  if (!selectedGuildId.value || !server.value || !rconCommand.value.trim()) return;
  try {
    const res = await nexusGamesService.rcon(
      selectedGuildId.value,
      server.value.id,
      rconCommand.value.trim(),
    );
    rconOutput.value = `> ${rconCommand.value}\n${res.response}\n\n${rconOutput.value}`;
    rconCommand.value = "";
  } catch (e) {
    rconOutput.value = `> ${rconCommand.value}\n[erreur] ${
      e instanceof Error ? e.message : "échec"
    }\n\n${rconOutput.value}`;
  }
}

// ── Ressources allouées ──
//
// Docker fige mémoire et processeur à la CRÉATION du conteneur : les changer
// n'a d'effet qu'à sa reconstruction, exactement comme la configuration. On le
// dit à l'écran plutôt que de laisser croire à un effet immédiat.

const memoryInput = ref<number>(0);
const cpuInput = ref<number>(2);
const savingResources = ref(false);

const resourcesChanged = computed(
  () =>
    !!server.value
    && (memoryInput.value !== server.value.allocated_memory_mb
      || cpuInput.value !== (server.value.cpu_limit ?? 0)),
);

function resetResourceInputs() {
  if (!server.value) return;
  memoryInput.value = server.value.allocated_memory_mb;
  cpuInput.value = server.value.cpu_limit ?? 2;
}

async function saveResources() {
  if (!selectedGuildId.value || !server.value || savingResources.value) return;
  savingResources.value = true;
  try {
    await nexusGamesService.updateResources(
      selectedGuildId.value,
      server.value.id,
      memoryInput.value,
      cpuInput.value,
    );
    success(
      isRunning.value
        ? "Ressources enregistrées. Elles seront appliquées au prochain arrêt puis démarrage."
        : "Ressources enregistrées. Elles seront appliquées au prochain démarrage.",
    );
    await load();
    resetResourceInputs();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Enregistrement impossible");
  } finally {
    savingResources.value = false;
  }
}

watch(
  [selectedGuildId, serverId],
  () => {
    void load().then(resetResourceInputs);
    // Les seuils vivent cote serveur : on les relit avec la fiche.
    void loadAlerts();
    void loadSchedule();
  },
  { immediate: true },
);
watch(onglet, (o) => {
  if (o === "logs") void loadLogs();
  if (o === "joueurs") void loadSessions();
  // L'historique n'est chargé qu'à l'ouverture de l'onglet : une journée de
  // mesures n'a pas à être demandée à quelqu'un venu lire les logs.
  if (o === "surveillance") void loadHistorique();
});

/// Mêmes sections, même ordre et mêmes contrôles que le formulaire de création.
const groupesConfig = useTemplateFieldGroups(computed(() => template.value?.config_schema));

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("fr-FR");
}

function fmtDuration(secs: number | null): string {
  if (secs === null) return "en cours";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return h > 0 ? `${h} h ${m} min` : `${m} min`;
}
</script>

<template>
  <AdminPageShell
    :title="server?.name ?? 'Serveur de jeu'"
    :subtitle="template?.name ?? ''"
  >
    <p v-if="errorMessage" class="sd-error">{{ errorMessage }}</p>
    <p v-else-if="loading && !server" class="sd-hint">Chargement…</p>

    <template v-else-if="server">
      <!-- Barre d'état + actions -->
      <div class="sd-bar">
        <span class="sd-status" :class="`st-${server.status}`">
          {{ STATUS_LABELS[server.status] ?? server.status }}
        </span>
        <button
          v-if="adresse"
          type="button"
          class="sd-port sd-copie"
          :title="`Copier ${adresse}`"
          @click="copier(adresse)"
        >
          {{ adresse }}
        </button>
        <span v-else-if="server.host_port" class="sd-port" title="Hote public non configure">
          Port {{ server.host_port }}
        </span>
        <span class="sd-mem">{{ server.allocated_memory_mb }} Mo</span>
        <span v-if="server.cpu_limit" class="sd-mem">{{ server.cpu_limit }} cœur(s)</span>

        <div class="sd-actions">
          <template v-if="isRunning">
            <button :disabled="busy" @click="act('stop')">Arrêter</button>
            <button :disabled="busy || isTransient" @click="act('restart')">Redémarrer</button>
          </template>
          <template v-else>
            <button :disabled="busy || isTransient" @click="act('start')">
              {{ isScheduled ? "Lancer maintenant" : "Démarrer" }}
            </button>
            <button :disabled="busy || isTransient" @click="startAndRevealIpNow">
              Démarrer + IP
            </button>
            <button
              :disabled="busy || isTransient"
              @click="showScheduleForm = !showScheduleForm"
            >
              {{ isScheduled ? "Reprogrammer" : "Programmer (Ouverture & Fermeture)" }}
            </button>
          </template>
          <AppButton variant="danger" size="sm" @click="remove">Supprimer</AppButton>
        </div>
      </div>

      <!-- Formulaire de programmation unifié (Ouverture + Fermeture) -->
      <div v-if="showScheduleForm" class="sd-schedule">
        <label>
          {{ isRunning ? "Révéler l’adresse le" : "Ouverture le" }}
          <input type="datetime-local" v-model="revealAtInput" />
        </label>
        <label>
          Fermeture le
          <input type="datetime-local" v-model="stopAtInput" />
        </label>
        <button :disabled="scheduling || !revealAtInput" @click="submitSchedule">
          {{ scheduling ? "Programmation…" : "Enregistrer la programmation" }}
        </button>
        <p class="sd-hint">
          {{
            isRunning
              ? "L’adresse sera révélée à l’heure d'ouverture choisie. Le serveur sera fermé à la date d'arrêt programmée."
              : "Le conteneur démarrera automatiquement ~5 min avant, et l’adresse sera révélée à l'heure d'ouverture. La fermeture et le planning seront automatiquement synchronisés."
          }}
        </p>
      </div>

      <p v-if="server.last_error" class="sd-lasterror">⚠ {{ server.last_error }}</p>

      <!-- Onglets -->
      <div class="sd-tabs">
        <button
          v-for="t in (['apercu', 'config', 'surveillance', 'logs', 'commandes', 'console', 'joueurs'] as Onglet[])"
          :key="t"
          type="button"
          :class="{ active: onglet === t }"
          @click="onglet = t"
        >
          {{
            { apercu: "Aperçu", config: "Configuration", surveillance: "Surveillance", logs: "Logs", commandes: "Commandes", console: "Console", joueurs: "Joueurs" }[t]
          }}
        </button>
      </div>

      <!-- Aperçu -->
      <section v-if="onglet === 'apercu'" class="sd-pane">
        <div v-if="stats" class="sd-stats">
          <div class="sd-stat">
            <span class="sd-stat-val">{{ stats.cpu_percent.toFixed(1) }} %</span>
            <span class="sd-stat-lbl">Processeur</span>
          </div>
          <div class="sd-stat">
            <span class="sd-stat-val">
              {{ stats.memory_used_mb }} / {{ stats.memory_limit_mb }} Mo
            </span>
            <span class="sd-stat-lbl">Mémoire</span>
          </div>
          <div class="sd-stat">
            <span class="sd-stat-val">{{ server.last_player_count }}</span>
            <span class="sd-stat-lbl">Joueurs connectés</span>
          </div>
        </div>
        <p v-else class="sd-hint">
          Statistiques disponibles uniquement quand le serveur tourne.
        </p>

        <dl class="sd-meta">
          <div><dt>Créé le</dt><dd>{{ fmtDate(server.created_at) }}</dd></div>
          <div><dt>Ouverture prévue</dt><dd>{{ fmtDate(server.ip_reveal_at) }}</dd></div>
          <div><dt>Fermeture prévue</dt><dd>{{ fmtDate(plannedStopAt) }}</dd></div>
          <div><dt>Démarré réellement</dt><dd>{{ fmtDate(server.started_at) }}</dd></div>
          <div><dt>Fermé réellement</dt><dd>{{ fmtDate(server.stopped_at) }}</dd></div>
          <div><dt>Dernière activité</dt><dd>{{ fmtDate(server.last_active_at) }}</dd></div>
          <div>
            <dt>Adresse</dt>
            <dd>
              {{ server.ip_revealed ? "révélée" : `masquée jusqu'au ${fmtDate(server.ip_reveal_at)}` }}
              <AppButton
                v-if="!server.ip_revealed"
                variant="warning"
                size="xs"
                :disabled="revealingIp || !isRunning || !server.host_port"
                :title="!isRunning ? 'Le serveur doit être en ligne' : undefined"
                @click="revealIpNow"
              >
                {{ revealingIp ? "Révélation…" : "Révéler maintenant" }}
              </AppButton>
            </dd>
          </div>
        </dl>
      </section>

      <!-- Pilotage dans le temps : plages OU permanence, jamais les deux -->
      <section v-if="onglet === 'apercu' && server" class="sd-pane sd-resources">
        <h3>Pilotage automatique</h3>
        <p class="sd-note">
          Deux façons de faire, au choix. Sans activation, rien ne change : c'est toi qui
          pilotes.
        </p>

        <div class="sd-modes">
          <label class="sd-mode" :class="{ 'sd-mode-actif': !estPermanence }">
            <input
              type="radio"
              name="schedule-mode"
              :checked="!estPermanence"
              @change="choisirMode('ranges')"
            />
            <span>
              <strong>Plages d'ouverture</strong>
              <small>
                Le serveur s'allume et s'éteint aux heures indiquées. Pour un serveur de
                soirée, qui n'a pas besoin de tourner la journée.
              </small>
            </span>
          </label>

          <label class="sd-mode" :class="{ 'sd-mode-actif': estPermanence }">
            <input
              type="radio"
              name="schedule-mode"
              :checked="estPermanence"
              @change="choisirMode('restart')"
            />
            <span>
              <strong>Permanence 24/24</strong>
              <small>
                Le serveur tourne en continu et redémarre à intervalle régulier : un jeu qui
                tourne des jours d'affilée ne rend pas la mémoire qu'il prend, et finit par
                ramer.
              </small>
            </span>
          </label>
        </div>

        <label class="sd-field sd-field-inline">
          <AppToggle v-model="scheduleEnabled" />
          <span>{{ estPermanence ? "Activer la permanence" : "Activer les plages horaires" }}</span>
        </label>

        <template v-if="scheduleEnabled">
          <div class="sd-form">
            <label class="sd-field">
              <span>Fuseau horaire</span>
              <input v-model="scheduleTimezone" type="text" placeholder="Europe/Paris" />
              <small class="sd-note">
                Les heures ci-dessous sont locales à ce fuseau. Le changement d'heure est
                suivi automatiquement.
              </small>
            </label>

            <label class="sd-field">
              <span>
                {{ estPermanence ? "Préavis avant redémarrage (minutes)" : "Préavis avant fermeture (minutes)" }}
              </span>
              <input v-model.number="scheduleWarn" type="number" min="0" max="120" />
              <small class="sd-note">
                <template v-if="estPermanence">
                  Annoncé dans le jeu <em>et</em> sur Discord. Une dernière annonce part
                  toujours dans le jeu une minute avant la coupure. 0 = pas de préavis
                  anticipé.
                </template>
                <template v-else>
                  Un message est envoyé dans le jeu. 0 = pas d'annonce.
                </template>
              </small>
            </label>
          </div>

          <!-- Permanence : cadence des redémarrages -->
          <div v-if="estPermanence" class="sd-form">
            <label class="sd-field">
              <span>Redémarrer toutes les</span>
              <select v-model.number="restartIntervalHours">
                <option v-for="h in restartIntervalChoices" :key="h" :value="h">
                  {{ h }} heure{{ h > 1 ? "s" : "" }}
                </option>
              </select>
              <small class="sd-note">
                Les redémarrages tombent à heure fixe, tous les jours : toutes les 6 h à
                partir de minuit, c'est 0h, 6h, 12h et 18h.
              </small>
            </label>

            <label class="sd-field">
              <span>À la minute</span>
              <input v-model.number="restartAnchorMinute" type="number" min="0" max="59" />
              <small class="sd-note">
                Pour décaler les créneaux de l'heure pile — utile quand plusieurs serveurs
                redémarreraient sinon en même temps.
              </small>
            </label>
          </div>

          <!-- Plages horaires -->
          <div v-else class="sd-ranges">
            <div v-for="(plage, index) in scheduleRanges" :key="index" class="sd-range-row">
              <div class="sd-range-hours">
                <input v-model="plage.start" type="time" />
                <span>→</span>
                <input v-model="plage.end" type="time" />
                <AppButton variant="secondary" size="xs" @click="retirerPlage(index)">
                  Retirer
                </AppButton>
              </div>
              <div class="sd-range-days">
                <button
                  v-for="jour in JOURS"
                  :key="jour.bit"
                  type="button"
                  class="sd-day"
                  :class="{ 'sd-day--on': jourActif(plage.days, jour.bit) }"
                  :aria-pressed="jourActif(plage.days, jour.bit)"
                  :title="jour.long"
                  @click="basculerJour(index, jour.bit)"
                >
                  {{ jour.court }}
                </button>
                <AppButton
                  variant="secondary"
                  size="xs"
                  @click="appliquerATousLesJours(index)"
                >
                  Tous les jours
                </AppButton>
              </div>
              <small v-if="plage.days === 0" class="sd-note">
                Aucun jour coché : cette plage n'ouvrira jamais.
              </small>
              <small v-else-if="plage.end <= plage.start" class="sd-note">
                Cette plage franchit minuit : elle se termine le lendemain matin,
                sans qu'il soit besoin de cocher le jour suivant.
              </small>
            </div>
            <p v-if="scheduleRanges.length === 0" class="sd-note">
              Aucune plage : ajoute-en une avant d'activer.
            </p>
            <AppButton variant="secondary" size="xs" @click="ajouterPlage">
              Ajouter une plage
            </AppButton>
          </div>

          <p v-if="estPermanence && prochainRedemarrage" class="sd-hint">
            Prochain redémarrage : {{ prochainRedemarrage }}.
          </p>
          <p v-if="!estPermanence && prochaineOuverture" class="sd-hint">
            Prochaine ouverture : {{ prochaineOuverture }}.
          </p>
          <p v-if="plannedStopAt" class="sd-hint">
            Après la date de fin de session, le serveur s'arrête et ne rouvre plus.
          </p>
        </template>

        <div class="sd-thresholds-row">
          <AppButton variant="secondary" size="sm" :disabled="savingSchedule" @click="saveSchedule">
            {{ savingSchedule ? "Enregistrement…" : "Enregistrer" }}
          </AppButton>
        </div>

        <p v-if="scheduleDisabledKeys.length" class="sd-hint">
          Redémarrage automatique du jeu désactivé ({{ scheduleDisabledKeys.join(", ") }}) : il
          ferait double emploi, et surtout il couperait sans prévenir personne.
        </p>
      </section>

      <!-- Ressources allouées -->
      <section v-if="onglet === 'apercu' && server" class="sd-pane sd-resources">
        <h3>Ressources allouées</h3>
        <div class="sd-form">
          <label class="sd-field">
            <span>Mémoire (Mo)</span>
            <span class="sd-slider">
              <input
                v-model.number="memoryInput"
                type="range"
                class="sd-range"
                :min="template?.min_memory_mb ?? 512"
                :max="template?.max_memory_mb ?? 16384"
                step="512"
              />
              <input
                v-model.number="memoryInput"
                type="number"
                class="sd-slider-value"
                :min="template?.min_memory_mb ?? 512"
                :max="template?.max_memory_mb ?? 16384"
                step="512"
              />
            </span>
            <small v-if="template" class="sd-note">
              Entre {{ template.min_memory_mb }} et {{ template.max_memory_mb }} Mo pour ce jeu.
            </small>
          </label>

          <label class="sd-field">
            <span>Cœurs processeur</span>
            <span class="sd-slider">
              <input
                v-model.number="cpuInput"
                type="range"
                class="sd-range"
                min="0.5"
                max="16"
                step="0.5"
              />
              <input
                v-model.number="cpuInput"
                type="number"
                class="sd-slider-value"
                min="0.5"
                max="16"
                step="0.5"
              />
            </span>
            <small class="sd-note">
              Plafond, pas une réservation : le serveur n'utilise que ce dont il a besoin.
              L'unité est le processeur logique (thread), comme pour Docker.
            </small>
          </label>
        </div>

        <p class="sd-hint">
          Docker fige ces limites à la création du conteneur : le changement s'applique au
          prochain démarrage, qui le reconstruit. Le monde et les sauvegardes sont conservés.
        </p>

        <div class="sd-thresholds-row">
          <AppButton
            variant="secondary"
            size="sm"
            :disabled="!resourcesChanged || savingResources"
            @click="saveResources"
          >
            {{ savingResources ? "Enregistrement…" : "Enregistrer les ressources" }}
          </AppButton>
          <AppButton
            v-if="resourcesChanged"
            variant="secondary"
            size="xs"
            @click="resetResourceInputs"
          >
            Annuler
          </AppButton>
        </div>
      </section>

      <!-- Configuration -->
      <section v-else-if="onglet === 'config'" class="sd-pane">
        <p v-if="!template?.config_schema?.length" class="sd-hint">
          Ce jeu n'expose aucun réglage modifiable.
        </p>
        <template v-else>
          <details v-for="g in groupesConfig" :key="g.nom" class="sd-group" open>
            <summary>
              {{ g.nom }}
              <span class="sd-group-count">{{ g.champs.length }}</span>
            </summary>
            <div class="sd-form">
              <GameConfigField
                v-for="f in g.champs"
                :key="f.key"
                :field="f"
                v-model="config[f.key]"
              />
            </div>
          </details>
          <AppButton variant="secondary" size="sm" :disabled="savingConfig" @click="saveConfig">
            {{ savingConfig ? "Enregistrement…" : "Enregistrer" }}
          </AppButton>
          <p class="sd-hint">
            Les changements prennent effet au prochain démarrage du serveur, qui
            reconstruit le conteneur pour les lui appliquer. Le monde et les
            sauvegardes sont conservés. Un serveur qui tourne garde ses réglages
            actuels jusqu'à son prochain arrêt puis démarrage.
          </p>
        </template>
      </section>

      <!-- Surveillance Système -->
      <section v-else-if="onglet === 'surveillance'" class="sd-pane">
        <div class="sd-col-header">
          <h3>📊 Surveillance système</h3>
          <span v-if="stats" class="sd-live-badge">En direct (5s)</span>
        </div>

        <!-- Choix de la fenêtre d'observation. Les courbes ne sont plus
             accumulées par la page : elles viennent de l'historique enregistré
             côté serveur, et couvrent donc du temps passé hors de cet écran. -->
        <div class="sd-plages">
          <button
            v-for="p in PLAGES"
            :key="p.secondes"
            class="sd-plage-btn"
            :class="{ active: plageChoisie === p.secondes }"
            :disabled="historiqueEnCours"
            @click="changerPlage(p.secondes)"
          >
            {{ p.libelle }}
          </button>
          <span v-if="pasLisible" class="sd-plage-pas">un point = {{ pasLisible }}</span>
        </div>

        <div v-if="stats" class="sd-surveillance-full-grid">
          <div class="sd-surv-card sd-surv-large">
            <div class="sd-surv-header">
              <span class="sd-surv-label">Processeur (CPU)</span>
              <span class="sd-surv-val">{{ stats.cpu_percent.toFixed(1) }} %</span>
            </div>
            <div class="sd-meter">
              <div class="sd-meter-bar" :style="{ width: `${Math.min(stats.cpu_percent, 100)}%` }"></div>
            </div>
            <div class="sd-chart-large-box">
              <Line :data="cpuChartData" :options="chartOptions" />
            </div>
          </div>

          <div class="sd-surv-card sd-surv-large">
            <div class="sd-surv-header">
              <span class="sd-surv-label">Mémoire RAM</span>
              <span class="sd-surv-val">{{ stats.memory_used_mb }} / {{ stats.memory_limit_mb }} Mo</span>
            </div>
            <div class="sd-meter">
              <div
                class="sd-meter-bar ram-bar"
                :style="{ width: `${Math.min((stats.memory_used_mb / Math.max(stats.memory_limit_mb, 1)) * 100, 100)}%` }"
              ></div>
            </div>
            <div class="sd-chart-large-box">
              <Line :data="ramChartData" :options="chartOptions" />
            </div>
          </div>

          <!-- Trafic réseau du conteneur. Docker donne des octets CUMULÉS
               depuis son démarrage : un total, pas un débit. On l'affiche tel
               quel plutôt que d'inventer une vitesse à partir d'une seule
               mesure — c'est la quantité échangée qui a du sens ici (un serveur
               qui n'échange rien n'a personne dessus). -->
          <!-- Le temps de réponse du jeu : c'est lui qui dit un lag. CPU et
               RAM disent ce que le conteneur consomme, pas ce que les joueurs
               ressentent — un serveur peut ramer à 30 % de processeur. -->
          <div class="sd-surv-card sd-surv-large">
            <div class="sd-surv-header">
              <span class="sd-surv-label">Temps de réponse du jeu</span>
              <span
                class="sd-surv-val"
                :style="{ color: (stats.rcon_latency_ms ?? 0) > 500 ? 'var(--danger)' : undefined }"
              >
                {{ stats.rcon_latency_ms === null ? "—" : `${stats.rcon_latency_ms} ms` }}
              </span>
            </div>
            <!-- Échelle libre : un temps de réponse n'a pas de plafond connu,
                 en imposer un écraserait la courbe ou masquerait un pic. -->
            <div class="sd-chart-large-box">
              <Line :data="latencyChartData" :options="chartOptionsAuto" />
            </div>
          </div>

          <div class="sd-surv-card sd-surv-large">
            <div class="sd-surv-header">
              <span class="sd-surv-label">Débit réseau</span>
              <span class="sd-surv-val">
                <template v-if="stats.network_rx_bytes_per_sec !== null">
                  ↓ {{ debit(stats.network_rx_bytes_per_sec) }} · ↑
                  {{ debit(stats.network_tx_bytes_per_sec ?? 0) }}
                </template>
                <template v-else>—</template>
              </span>
            </div>
            <!-- Reçu et envoyé sur le MÊME graphe : c'est l'écart entre les
                 deux qui parle. Un serveur de jeu émet bien plus qu'il ne
                 reçoit, et une émission qui plafonne fait laguer tout le monde. -->
            <div class="sd-chart-large-box">
              <Line :data="netChartData" :options="chartOptionsReseau" />
            </div>
          </div>

          <!-- Volumes cumulés depuis le démarrage du conteneur. Sans courbe :
               un compteur cumulé ne peut que monter, puis retombe à zéro au
               redémarrage — et les serveurs redémarrent chaque nuit. La courbe
               occupait la place d'une information, sans en être une. Le débit
               instantané, lui, garde son graphe : c'est là que se voit une
               saturation. -->
          <div class="sd-surv-card">
            <div class="sd-surv-label">Volume échangé</div>
            <div class="sd-surv-val">
              ↓ {{ volume(stats.network_rx_bytes) }} · ↑ {{ volume(stats.network_tx_bytes) }}
            </div>
            <p class="sd-note">Depuis le démarrage du conteneur.</p>
          </div>

          <div class="sd-surv-card sd-surv-large">
            <div class="sd-surv-header">
              <span class="sd-surv-label">Joueurs en jeu</span>
              <span class="sd-surv-val">{{ server.last_player_count }}</span>
            </div>
            <!-- La carte n'affichait qu'un chiffre, presque toujours « 0 ».
                 La courbe dit à quelle heure le serveur est utilisé, ce que la
                 valeur de l'instant ne pourra jamais raconter. -->
            <div class="sd-chart-large-box">
              <Line :data="playersChartData" :options="chartOptionsJoueurs" />
            </div>
          </div>

          <div class="sd-surv-card">
            <div class="sd-surv-label">Statut conteneur</div>
            <div class="sd-surv-val">
              <span class="sd-status" :class="`st-${server.status}`">
                {{ STATUS_LABELS[server.status] ?? server.status }}
              </span>
            </div>
          </div>
        </div>

        <div v-else class="sd-surv-empty">
          <p v-if="!isRunning">Le serveur est éteint. Démarrez-le pour observer le processeur et la mémoire RAM en direct.</p>
          <p v-else>Mesure des ressources du conteneur en cours…</p>
        </div>

        <!-- Configuration des Alertes Webhook Discord -->
        <div class="sd-webhook-card sd-surv-card" style="margin-top: 2rem;">
          <h4>🔔 Alertes Webhook Discord</h4>
          <p class="sd-note">Recevez une notification automatique sur Discord lorsque le CPU, la RAM ou le temps de réponse dépasse les seuils.</p>
          <div class="sd-webhook-form">
            <label class="sd-field">
              <span>
                URL du Webhook Discord
                <template v-if="alertsConfigured"> — déjà enregistré</template>
              </span>
              <!-- Jamais pré-rempli : l'URL est un secret, elle ne revient pas
                   du serveur. Laisser vide conserve celle enregistrée. -->
              <input
                v-model="webhookUrl"
                type="url"
                :placeholder="alertsConfigured
                  ? 'Laisser vide pour conserver le webhook enregistré'
                  : 'https://discord.com/api/webhooks/...'"
              />
            </label>
            <div class="sd-thresholds-row">
              <label class="sd-field">
                <span>Seuil CPU (%)</span>
                <input v-model.number="cpuThreshold" type="number" min="1" max="100" />
              </label>
              <label class="sd-field">
                <span>Seuil RAM (%)</span>
                <input v-model.number="ramThreshold" type="number" min="1" max="100" />
              </label>
              <label class="sd-field">
                <span>Seuil temps de réponse (ms)</span>
                <input v-model.number="latencyThreshold" type="number" min="50" max="10000" step="50" />
              </label>
            </div>
            <p class="sd-hint">
              Le temps de réponse est la mesure qui correspond au lag ressenti : un serveur
              peut ramer à 30 % de processeur. La surveillance tourne côté serveur, toutes
              les minutes — page fermée comprise — et n'envoie pas deux fois la même alerte
              à moins de cinq minutes d'intervalle.
            </p>
            <div class="sd-thresholds-row">
              <AppButton
                variant="secondary"
                size="sm"
                :disabled="savingAlerts"
                @click="saveAlertSettings"
              >
                {{ savingAlerts ? "Enregistrement…" : "Enregistrer l'alerte" }}
              </AppButton>
              <AppButton
                v-if="alertsConfigured"
                variant="warning"
                size="sm"
                @click="disableAlerts"
              >
                Arrêter la surveillance
              </AppButton>
            </div>
          </div>
        </div>
      </section>

      <!-- Logs -->
      <section v-else-if="onglet === 'logs'" class="sd-pane">
        <div class="sd-col-header">
          <h3>📜 Logs du conteneur</h3>
          <AppButton variant="ghost" size="sm" @click="loadLogs">Rafraîchir</AppButton>
        </div>
        <pre class="sd-logs full-width-logs">{{ logs.join("\n") || "Aucune ligne de log disponible." }}</pre>
      </section>

      <!-- Console RCON -->
      <!-- Commandes d'administration, déclarées par le jeu -->
      <section v-if="onglet === 'commandes'" class="sd-pane">
        <p v-if="!template?.supports_rcon" class="sd-hint">
          Ce jeu ne supporte pas RCON : aucune commande ne peut lui être envoyée.
        </p>
        <GameCommandPanel
          v-else-if="selectedGuildId && server"
          :guild-id="selectedGuildId"
          :server-id="server.id"
          :running="isRunning"
        />
      </section>

      <section v-else-if="onglet === 'console'" class="sd-pane">
        <p v-if="!template?.supports_rcon" class="sd-hint">
          Ce jeu ne supporte pas RCON.
        </p>
        <p v-else-if="!isRunning" class="sd-hint">
          Démarre le serveur pour lui envoyer des commandes.
        </p>
        <template v-else>
          <form class="sd-rcon" @submit.prevent="sendRcon">
            <input v-model="rconCommand" type="text" placeholder="say Bonjour" />
            <button type="submit">Envoyer</button>
          </form>
          <pre class="sd-logs">{{ rconOutput || "Aucune commande envoyée." }}</pre>
        </template>
      </section>

      <!-- Joueurs -->
      <section v-else class="sd-pane">
        <p v-if="!sessions.length" class="sd-hint">Aucune session enregistrée.</p>
        <table v-else class="sd-table">
          <thead>
            <tr><th>Joueur</th><th>Connexion</th><th>Déconnexion</th><th>Durée</th></tr>
          </thead>
          <tbody>
            <tr v-for="s in sessions" :key="s.id">
              <td>{{ s.player_name }}</td>
              <td>{{ fmtDate(s.joined_at) }}</td>
              <td>{{ fmtDate(s.left_at) }}</td>
              <td>{{ fmtDuration(s.duration_seconds) }}</td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-if="sessions.length"
          :current-page="sessionsPage"
          :total-pages="sessionsPages"
          :total-items="sessionsTotal"
          :per-page="sessionsParPage"
          @update:current-page="allerALaPageDeSessions($event)"
          @update:per-page="changerTailleDeSessions($event)"
        />
      </section>
    </template>
  </AdminPageShell>
</template>

<style scoped src="../../styles/nexus-server-detail.css"></style>
