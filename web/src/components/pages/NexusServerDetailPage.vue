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
import { useAuth } from "../../composables/useAuth";
import { useToast } from "../../composables/useToast";
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

const route = useRoute();
const router = useRouter();
const { selectedGuildId } = useGuildSelector();
const { user } = useAuth();
const { success, error: showError } = useToast();

const serverId = computed(() => String(route.params.id ?? ""));

const server = ref<GameServer | null>(null);
const config = ref<Record<string, string>>({});
const template = ref<GameTemplate | null>(null);
const stats = ref<GameServerStats | null>(null);
const logs = ref<string[]>([]);
const sessions = ref<PlayerSession[]>([]);

const loading = ref(false);
const errorMessage = ref("");
const busy = ref(false);
const savingConfig = ref(false);
const revealingIp = ref(false);
const showScheduleForm = ref(false);
const scheduling = ref(false);
/// Valeur du champ `datetime-local` (heure locale « YYYY-MM-DDTHH:mm »).
const revealAtInput = ref("");
const rconCommand = ref("");
const rconOutput = ref("");

type Onglet = "apercu" | "config" | "logs" | "console" | "joueurs";
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
    await nexusGamesService[action](selectedGuildId.value, server.value.id, user.value?.id ?? "");
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
    await nexusGamesService.remove(selectedGuildId.value, server.value.id, user.value?.id ?? "");
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
    await nexusGamesService.revealIp(
      selectedGuildId.value,
      server.value.id,
      user.value?.id ?? "",
    );
    success("Adresse révélée immédiatement.");
    await load();
  } catch (e) {
    showError(e instanceof Error ? e.message : "Révélation impossible");
  } finally {
    revealingIp.value = false;
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
        user.value?.id ?? "",
      );
      success("Révélation de l'adresse programmée.");
    } else {
      await nexusGamesService.schedule(
        selectedGuildId.value,
        server.value.id,
        iso,
        user.value?.id ?? "",
      );
      success("Ouverture programmée : les inscriptions sont ouvertes.");
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
      user.value?.id ?? "",
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

// ── Statistiques en direct, uniquement quand le serveur tourne ──
let statsTimer: ReturnType<typeof setInterval> | null = null;

async function refreshStats() {
  if (!selectedGuildId.value || !server.value || !isRunning.value) {
    stats.value = null;
    return;
  }
  stats.value = await nexusGamesService
    .stats(selectedGuildId.value, server.value.id)
    .catch(() => null);
}

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
    <p v-else-if="loading" class="sd-hint">Chargement…</p>

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
            <button
              :disabled="busy || isTransient"
              @click="showScheduleForm = !showScheduleForm"
            >
              {{ isScheduled ? "Reprogrammer" : "Programmer l’ouverture" }}
            </button>
          </template>
          <AppButton variant="danger" size="sm" @click="remove">Supprimer</AppButton>
        </div>
      </div>

      <!-- Formulaire de programmation (Préparation / révélation auto) -->
      <div v-if="showScheduleForm" class="sd-schedule">
        <label>
          {{ isRunning ? "Révéler l’adresse le" : "Ouverture le" }}
          <input type="datetime-local" v-model="revealAtInput" />
        </label>
        <button :disabled="scheduling || !revealAtInput" @click="submitSchedule">
          {{ scheduling ? "Programmation…" : "Programmer" }}
        </button>
        <p class="sd-hint">
          {{
            isRunning
              ? "L’adresse sera révélée automatiquement à l’heure choisie."
              : "Le conteneur démarrera automatiquement ~5 min avant, et l’adresse sera révélée à l’heure choisie. Les salons et le panneau d’inscription sont créés dès maintenant."
          }}
        </p>
      </div>

      <p v-if="server.last_error" class="sd-lasterror">⚠ {{ server.last_error }}</p>

      <!-- Onglets -->
      <div class="sd-tabs">
        <button
          v-for="t in (['apercu', 'config', 'logs', 'console', 'joueurs'] as Onglet[])"
          :key="t"
          type="button"
          :class="{ active: onglet === t }"
          @click="onglet = t"
        >
          {{
            { apercu: "Aperçu", config: "Configuration", logs: "Logs", console: "Console", joueurs: "Joueurs" }[t]
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
          <div><dt>Démarré le</dt><dd>{{ fmtDate(server.started_at) }}</dd></div>
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

      <!-- Logs -->
      <section v-else-if="onglet === 'logs'" class="sd-pane">
        <AppButton variant="ghost" size="sm" @click="loadLogs">Rafraîchir</AppButton>
        <pre class="sd-logs">{{ logs.join("\n") || "Aucune ligne." }}</pre>
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
