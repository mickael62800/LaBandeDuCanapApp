<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
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
  type GameServerStats,
  type GameTemplate,
  type PlayerSession,
} from "@/services/nexusGamesService";
import { useTemplateFieldGroups } from "@/composables/useTemplateFieldGroups";
import GameConfigField from "../molecules/GameConfigField.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import GameCommandPanel from "../organisms/GameCommandPanel.vue";

import { Line } from 'vue-chartjs'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
} from 'chart.js'

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  Filler
);

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
const stats = ref<GameServerStats | null>(null);
const logs = ref<string[]>([]);
const sessions = ref<PlayerSession[]>([]);
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
  try {
    await nexusGamesService.remove(selectedGuildId.value, server.value.id);
    success("Serveur supprimé.");
    router.push("/nexus/servers");
  } catch (e) {
    showError(e instanceof Error ? e.message : "Suppression impossible");
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
  sessions.value = await nexusGamesService
    .sessions(selectedGuildId.value, server.value.id)
    .catch(() => []);
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

// ── Statistiques en direct + historique graphique, uniquement quand le serveur tourne ──
let statsTimer: ReturnType<typeof setInterval> | null = null;
const cpuHistory = ref<number[]>([]);
const ramHistory = ref<number[]>([]);

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

// ── Alertes de supervision ──
//
// Elles vivaient dans le navigateur : seuils et webhook en `localStorage`,
// vérification à chaque rafraîchissement de la page. Fermer l'onglet arrêtait
// donc la surveillance — or c'est la nuit, page fermée, qu'un serveur sature.
//
// Tout est passé côté serveur. L'URL du webhook est un secret : elle ne
// revient jamais ici, l'écran sait seulement qu'un webhook est enregistré.

const cpuThreshold = ref<number>(85);
const ramThreshold = ref<number>(90);
/// Seuil de temps de réponse : la mesure qui correspond au lag ressenti.
/// CPU et RAM disent ce que le conteneur consomme, celle-ci ce que les joueurs
/// subissent — un serveur peut ramer à 30 % de processeur.
const latencyThreshold = ref<number>(500);
const webhookUrl = ref<string>("");
const alertsConfigured = ref(false);
const savingAlerts = ref(false);

async function loadAlertSettings() {
  if (!selectedGuildId.value || !serverId.value) return;
  try {
    const settings = await nexusGamesService.getAlertSettings(
      selectedGuildId.value,
      serverId.value,
    );
    alertsConfigured.value = settings.configured;
    cpuThreshold.value = settings.cpu_threshold;
    ramThreshold.value = settings.ram_threshold;
    latencyThreshold.value = settings.latency_threshold_ms;
  } catch {
    // Réglages indisponibles : on garde les valeurs par défaut affichées
    // plutôt que de vider le formulaire sous les yeux de l'administrateur.
  }
}

async function saveAlertSettings() {
  if (!selectedGuildId.value || !serverId.value || savingAlerts.value) return;
  savingAlerts.value = true;
  try {
    await nexusGamesService.saveAlertSettings(selectedGuildId.value, serverId.value, {
      // Champ laissé vide = on garde le webhook déjà enregistré.
      webhook_url: webhookUrl.value.trim() || undefined,
      cpu_threshold: cpuThreshold.value,
      ram_threshold: ramThreshold.value,
      latency_threshold_ms: latencyThreshold.value,
    });
    webhookUrl.value = "";
    success("Alertes enregistrées. La surveillance tourne côté serveur, page fermée comprise.");
    await loadAlertSettings();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Enregistrement impossible");
  } finally {
    savingAlerts.value = false;
  }
}

async function disableAlerts() {
  if (!selectedGuildId.value || !serverId.value) return;
  if (!confirm("Arrêter la surveillance de ce serveur ? Le webhook enregistré sera supprimé.")) {
    return;
  }
  try {
    await nexusGamesService.deleteAlertSettings(selectedGuildId.value, serverId.value);
    success("Surveillance arrêtée.");
    await loadAlertSettings();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Arrêt impossible");
  }
}


async function refreshStats() {
  if (!selectedGuildId.value || !server.value || !isRunning.value) {
    stats.value = null;
    cpuHistory.value = [];
    ramHistory.value = [];
    return;
  }
  const newStats = await nexusGamesService
    .stats(selectedGuildId.value, server.value.id)
    .catch(() => null);

  stats.value = newStats;

  if (newStats) {
    cpuHistory.value.push(newStats.cpu_percent);
    if (cpuHistory.value.length > 20) cpuHistory.value.shift();

    const ramPct = (newStats.memory_used_mb / Math.max(newStats.memory_limit_mb, 1)) * 100;
    ramHistory.value.push(ramPct);
    if (ramHistory.value.length > 20) ramHistory.value.shift();

    // Les seuils ne sont plus verifies ici : la surveillance tourne cote
    // serveur, page fermee comprise.
  }
}

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  animation: { duration: 0 },
  scales: {
    y: { 
      min: 0, 
      max: 100, 
      grid: { color: 'rgba(255, 255, 255, 0.1)' },
      ticks: { color: 'rgba(255, 255, 255, 0.5)' }
    },
    x: { display: false }
  },
  plugins: { legend: { display: false } },
  elements: {
    point: { radius: 0 }
  }
};

const cpuChartData = computed(() => {
  return {
    labels: cpuHistory.value.map((_, i) => i.toString()),
    datasets: [
      {
        label: 'CPU (%)',
        backgroundColor: 'rgba(52, 152, 219, 0.2)',
        borderColor: '#3498db',
        data: [...cpuHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      }
    ]
  };
});

const ramChartData = computed(() => {
  return {
    labels: ramHistory.value.map((_, i) => i.toString()),
    datasets: [
      {
        label: 'RAM (%)',
        backgroundColor: 'rgba(241, 196, 15, 0.2)',
        borderColor: '#f1c40f',
        data: [...ramHistory.value],
        fill: true,
        tension: 0.4,
        borderWidth: 2,
      }
    ]
  };
});

function syncStatsTimer() {
  if (statsTimer) {
    clearInterval(statsTimer);
    statsTimer = null;
  }
  if (isRunning.value) {
    void refreshStats();
    statsTimer = setInterval(refreshStats, 5000);
  } else {
    stats.value = null;
    cpuHistory.value = [];
    ramHistory.value = [];
  }
}

watch(isRunning, syncStatsTimer, { immediate: true });
onUnmounted(() => statsTimer && clearInterval(statsTimer));

watch(
  [selectedGuildId, serverId],
  () => {
    void load().then(resetResourceInputs);
    // Les seuils vivent cote serveur : on les relit avec la fiche.
    void loadAlertSettings();
  },
  { immediate: true },
);
watch(onglet, (o) => {
  if (o === "logs") void loadLogs();
  if (o === "joueurs") void loadSessions();
});

/// Mêmes sections, même ordre et mêmes contrôles que le formulaire de création.
const groupesConfig = useTemplateFieldGroups(computed(() => template.value?.config_schema));

/**
 * Rend un volume d'octets lisible : 2 300 000 000 ne se lit pas, « 2,14 Go »
 * oui.
 */
function volume(octets: number | null | undefined): string {
  const v = Number(octets) || 0;
  if (v < 1024) return `${v} o`;
  if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} Ko`;
  if (v < 1024 * 1024 * 1024) return `${(v / (1024 * 1024)).toFixed(1)} Mo`;
  return `${(v / (1024 * 1024 * 1024)).toFixed(2)} Go`;
}

/** Rend un débit lisible : 2 300 000 o/s ne se lit pas, « 2,19 Mo/s » oui. */
function debit(octetsParSeconde: number): string {
  const v = Number(octetsParSeconde) || 0;
  if (v < 1024) return `${v} o/s`;
  if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} Ko/s`;
  return `${(v / (1024 * 1024)).toFixed(2)} Mo/s`;
}

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
          <div class="sd-surv-card">
            <div class="sd-surv-label">Temps de réponse du jeu</div>
            <div
              class="sd-surv-val"
              :style="{ color: (stats.rcon_latency_ms ?? 0) > 500 ? 'var(--danger)' : undefined }"
            >
              {{ stats.rcon_latency_ms === null ? "—" : `${stats.rcon_latency_ms} ms` }}
            </div>
          </div>

          <div class="sd-surv-card">
            <div class="sd-surv-label">Débit réseau</div>
            <div class="sd-surv-val">
              <template v-if="stats.network_rx_bytes_per_sec !== null">
                ↓ {{ debit(stats.network_rx_bytes_per_sec) }} · ↑
                {{ debit(stats.network_tx_bytes_per_sec ?? 0) }}
              </template>
              <template v-else>—</template>
            </div>
          </div>

          <div class="sd-surv-card">
            <div class="sd-surv-label">Réseau reçu</div>
            <div class="sd-surv-val">{{ volume(stats.network_rx_bytes) }}</div>
          </div>

          <div class="sd-surv-card">
            <div class="sd-surv-label">Réseau envoyé</div>
            <div class="sd-surv-val">{{ volume(stats.network_tx_bytes) }}</div>
          </div>

          <div class="sd-surv-card">
            <div class="sd-surv-label">Joueurs en jeu</div>
            <div class="sd-surv-val">{{ server.last_player_count }}</div>
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
      </section>
    </template>
  </AdminPageShell>
</template>

<style scoped src="../../styles/nexus-server-detail.css"></style>
