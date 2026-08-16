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

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
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
  type TemplateField,
} from "@/services/nexusGamesService";
import AdminPageShell from "../layouts/AdminPageShell.vue";

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

type Onglet = "apercu" | "config" | "surveillance" | "logs" | "console" | "joueurs";
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
    await nexusGamesService.updateConfig(
      selectedGuildId.value,
      server.value.id,
      config.value,
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

// ── Alertes Webhook Discord ──
const cpuThreshold = ref<number>(85);
const ramThreshold = ref<number>(90);
const webhookUrl = ref<string>("");
const webhookCooldownMs = 5 * 60 * 1000; // 5 min de cooldown anti-spam par métrique
let lastCpuAlertTime = 0;
let lastRamAlertTime = 0;

// Charger les paramètres Webhook depuis le localStorage
onMounted(() => {
  const savedUrl = localStorage.getItem("nexus_webhook_url");
  if (savedUrl) webhookUrl.value = savedUrl;
  const savedCpu = localStorage.getItem("nexus_cpu_threshold");
  if (savedCpu) cpuThreshold.value = Number(savedCpu) || 85;
  const savedRam = localStorage.getItem("nexus_ram_threshold");
  if (savedRam) ramThreshold.value = Number(savedRam) || 90;
});

function saveAlertSettings() {
  localStorage.setItem("nexus_webhook_url", webhookUrl.value.trim());
  localStorage.setItem("nexus_cpu_threshold", cpuThreshold.value.toString());
  localStorage.setItem("nexus_ram_threshold", ramThreshold.value.toString());
  success("Paramètres d'alerte Webhook enregistrés.");
}

async function triggerDiscordWebhook(title: string, message: string, color: number) {
  if (!webhookUrl.value.trim()) return;
  try {
    await fetch(webhookUrl.value.trim(), {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        username: "Nexus Games · Alerte Serveur",
        embeds: [
          {
            title: `⚠️ ${title}`,
            description: message,
            color,
            fields: [
              { name: "Serveur", value: server.value?.name ?? "Serveur de jeu", inline: true },
              { name: "Statut", value: server.value?.status ?? "Inconnu", inline: true },
            ],
            timestamp: new Date().toISOString(),
          },
        ],
      }),
    });
  } catch (e) {
    console.warn("Échec envoi webhook alerte:", e);
  }
}

async function checkAlertThresholds(newStats: GameServerStats) {
  const now = Date.now();
  const ramPct = (newStats.memory_used_mb / Math.max(newStats.memory_limit_mb, 1)) * 100;

  // Alerte CPU
  if (newStats.cpu_percent >= cpuThreshold.value && now - lastCpuAlertTime > webhookCooldownMs) {
    lastCpuAlertTime = now;
    void triggerDiscordWebhook(
      "Dépassement de Seuil CPU",
      `Le serveur **${server.value?.name}** consomme **${newStats.cpu_percent.toFixed(1)}%** de CPU (seuil configuré: ${cpuThreshold.value}%).`,
      0xe74c3c,
    );
  }

  // Alerte RAM
  if (ramPct >= ramThreshold.value && now - lastRamAlertTime > webhookCooldownMs) {
    lastRamAlertTime = now;
    void triggerDiscordWebhook(
      "Dépassement de Seuil RAM",
      `Le serveur **${server.value?.name}** consomme **${ramPct.toFixed(1)}%** de sa mémoire RAM (**${newStats.memory_used_mb} Mo** / ${newStats.memory_limit_mb} Mo) (seuil configuré: ${ramThreshold.value}%).`,
      0xe67e22,
    );
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

    // Vérification des seuils
    void checkAlertThresholds(newStats);
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
        data: cpuHistory.value,
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
        data: ramHistory.value,
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

watch([selectedGuildId, serverId], load, { immediate: true });
watch(onglet, (o) => {
  if (o === "logs") void loadLogs();
  if (o === "joueurs") void loadSessions();
});

/// Mêmes sections que le formulaire de création.
const groupesConfig = computed(() => {
  const out: { nom: string; champs: TemplateField[] }[] = [];
  for (const f of template.value?.config_schema ?? []) {
    const nom = f.group || "Réglages";
    let g = out.find((x) => x.nom === nom);
    if (!g) {
      g = { nom, champs: [] };
      out.push(g);
    }
    g.champs.push(f);
  }
  return out;
});

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
          v-for="t in (['apercu', 'config', 'surveillance', 'logs', 'console', 'joueurs'] as Onglet[])"
          :key="t"
          type="button"
          :class="{ active: onglet === t }"
          @click="onglet = t"
        >
          {{
            { apercu: "Aperçu", config: "Configuration", surveillance: "Surveillance", logs: "Logs", console: "Console", joueurs: "Joueurs" }[t]
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

      <!-- Configuration -->
      <section v-else-if="onglet === 'config'" class="sd-pane">
        <p v-if="!template?.config_schema?.length" class="sd-hint">
          Ce jeu n'expose aucun réglage modifiable.
        </p>
        <template v-else>
          <details v-for="g in groupesConfig" :key="g.nom" class="sd-group" open>
            <summary>{{ g.nom }}</summary>
            <div class="sd-form">
            <label v-for="f in g.champs" :key="f.key" class="sd-field">
              <span>{{ f.label || f.key }}</span>
              <select v-if="f.type === 'enum'" v-model="config[f.key]">
                <option v-for="o in f.options ?? []" :key="o" :value="o">{{ o }}</option>
              </select>
              <input
                v-else-if="f.type === 'number'"
                v-model="config[f.key]"
                type="number"
                :min="f.min"
                :max="f.max"
              />
              <input v-else v-model="config[f.key]" type="text" :maxlength="f.max_length" />
              <small v-if="f.description" class="sd-note">{{ f.description }}</small>
            </label>
            </div>
          </details>
          <AppButton variant="secondary" size="sm" :disabled="savingConfig" @click="saveConfig">
            {{ savingConfig ? "Enregistrement…" : "Enregistrer" }}
          </AppButton>
          <p class="sd-hint">
            Les changements prennent effet au prochain redémarrage du serveur.
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
          <p class="sd-note">Recevez une notification automatique sur Discord lorsque le CPU ou la RAM dépasse les seuils.</p>
          <div class="sd-webhook-form">
            <label class="sd-field">
              <span>URL du Webhook Discord</span>
              <input v-model="webhookUrl" type="url" placeholder="https://discord.com/api/webhooks/..." />
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
            </div>
            <AppButton variant="secondary" size="sm" @click="saveAlertSettings">
              Enregistrer l'alerte
            </AppButton>
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
