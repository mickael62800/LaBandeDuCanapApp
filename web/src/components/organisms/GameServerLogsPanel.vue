<script setup lang="ts">
/**
 * Console et visualiseur enrichi de logs de serveur de jeu (Nexus).
 *
 * Fonctionnalités :
 * - Coloration syntaxique et interprétation des codes couleur ANSI en HTML sécurisé.
 * - Détection automatique des niveaux de sévérité (ERROR, WARN, SUCCESS, INFO, DEBUG) avec badges et bordures.
 * - Filtrage par niveau de log et catégorie (Joueurs, Sauvegardes, Réseau).
 * - Recherche textuelle instantanée avec surbrillance des correspondances.
 * - Suivi automatique du défilement (Auto-scroll) intelligent.
 * - Auto-refresh paramétrable (Off, 3s, 5s, 10s, 30s) avec indicateur de pulsation en direct.
 * - Sélecteur de volume de lignes (100, 300, 500, 1000).
 * - Outils d'export : Copier dans le presse-papier, Télécharger en fichier .log.
 * - Mode plein écran / agrandi pour analyse approfondie.
 */
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import AppButton from "@/components/atoms/AppButton.vue";
import AppBadge from "@/components/atoms/AppBadge.vue";
import { useToast } from "@/composables/useToast";
import { nexusGamesService } from "@/services/nexusGamesService";
import {
  parseLogLines,
  type LogCategory,
  type LogLevel,
  type ParsedLogLine,
} from "@/utils/logParser";

const props = defineProps<{
  guildId: string;
  serverId: string;
  serverName?: string;
  isRunning?: boolean;
}>();

const { success, error: showError } = useToast();

const rawLines = ref<string[]>([]);
const loading = ref(false);
const loadError = ref("");
const lastRefreshTime = ref<Date | null>(null);

// Filtres et options
const search = ref("");
const selectedLevel = ref<"all" | LogLevel>("all");
const selectedCategory = ref<"all" | LogCategory>("all");
const lineLimit = ref<number>(300);
const autoRefreshInterval = ref<number>(0); // 0 = désactivé, sinon en secondes
const autoScroll = ref<boolean>(true);
const isFullscreen = ref<boolean>(false);
const showLineNumbers = ref<boolean>(true);
const fontSize = ref<"sm" | "md" | "lg">("md");

// Référence du conteneur de logs pour le défilement
const terminalRef = ref<HTMLElement | null>(null);
let refreshTimer: ReturnType<typeof setInterval> | null = null;

// Lignes parsées et enrichies
const parsedLines = computed<ParsedLogLine[]>(() => {
  return parseLogLines(rawLines.value, search.value);
});

// Statistiques en temps réel
const stats = computed(() => {
  let errors = 0;
  let warnings = 0;
  let successes = 0;
  let players = 0;

  for (const line of parsedLines.value) {
    if (line.level === "error") errors++;
    else if (line.level === "warn") warnings++;
    else if (line.level === "success") successes++;

    if (line.category === "player") players++;
  }

  return {
    total: parsedLines.value.length,
    errors,
    warnings,
    successes,
    players,
  };
});

// Lignes visibles après application des filtres
const visibleLines = computed<ParsedLogLine[]>(() => {
  return parsedLines.value.filter((line) => {
    // Filtre par niveau
    if (selectedLevel.value !== "all" && line.level !== selectedLevel.value) {
      return false;
    }
    // Filtre par catégorie
    if (selectedCategory.value !== "all" && line.category !== selectedCategory.value) {
      return false;
    }
    // Filtre textuel
    if (search.value.trim()) {
      const q = search.value.trim().toLowerCase();
      if (!line.raw.toLowerCase().includes(q)) {
        return false;
      }
    }
    return true;
  });
});

async function fetchLogs(silent = false) {
  if (!props.guildId || !props.serverId) return;
  if (!silent) {
    loading.value = true;
  }
  loadError.value = "";

  try {
    const data = await nexusGamesService.logs(
      props.guildId,
      props.serverId,
      lineLimit.value,
    );
    rawLines.value = data;
    lastRefreshTime.value = new Date();

    if (autoScroll.value) {
      void scrollToBottom();
    }
  } catch (err) {
    loadError.value = err instanceof Error ? err.message : "Erreur lors de la récupération des logs";
  } finally {
    loading.value = false;
  }
}

async function scrollToBottom() {
  await nextTick();
  if (terminalRef.value) {
    terminalRef.value.scrollTop = terminalRef.value.scrollHeight;
  }
}

function handleScroll() {
  if (!terminalRef.value) return;
  const { scrollTop, scrollHeight, clientHeight } = terminalRef.value;
  // Si l'utilisateur remonte manuellement de plus de 40px, on désactive l'auto-scroll
  const atBottom = scrollHeight - (scrollTop + clientHeight) < 40;
  if (!atBottom && autoScroll.value) {
    autoScroll.value = false;
  }
}

function toggleAutoScroll() {
  autoScroll.value = !autoScroll.value;
  if (autoScroll.value) {
    void scrollToBottom();
  }
}

function copyLogs() {
  if (visibleLines.value.length === 0) return;
  const content = visibleLines.value.map((l) => l.raw).join("\n");
  navigator.clipboard.writeText(content).then(
    () => success(`${visibleLines.value.length} lignes copiées dans le presse-papier`),
    () => showError("Échec de la copie dans le presse-papier"),
  );
}

function downloadLogs() {
  if (visibleLines.value.length === 0) return;
  const content = visibleLines.value.map((l) => l.raw).join("\n");
  const blob = new Blob([content], { type: "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  const name = (props.serverName || props.serverId).replace(/[^a-z0-9_-]/gi, "_");
  const dateStr = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
  a.href = url;
  a.download = `logs-${name}-${dateStr}.log`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

function clearDisplay() {
  rawLines.value = [];
}

function setupAutoRefresh() {
  if (refreshTimer) {
    clearInterval(refreshTimer);
    refreshTimer = null;
  }
  if (autoRefreshInterval.value > 0) {
    refreshTimer = setInterval(() => {
      void fetchLogs(true);
    }, autoRefreshInterval.value * 1000);
  }
}

watch(autoRefreshInterval, setupAutoRefresh);
watch(lineLimit, () => void fetchLogs());
watch(
  () => [props.guildId, props.serverId],
  () => void fetchLogs(),
  { immediate: true },
);

onMounted(() => {
  setupAutoRefresh();
});

onUnmounted(() => {
  if (refreshTimer) {
    clearInterval(refreshTimer);
  }
});
</script>

<template>
  <div class="gsl-container" :class="{ 'gsl-fullscreen': isFullscreen }">
    <!-- En-tête : Titre, Statut & Statistiques en temps réel -->
    <header class="gsl-header">
      <div class="gsl-title-group">
        <div class="gsl-title">
          <span class="gsl-icon">📜</span>
          <h3>Console des Logs</h3>
          <AppBadge
            v-if="isRunning !== undefined"
            :label="isRunning ? 'En ligne' : 'Arrêté'"
            :variant="isRunning ? 'success' : 'default'"
          />
        </div>
        <span v-if="lastRefreshTime" class="gsl-subtitle">
          Dernière actualisation à {{ lastRefreshTime.toLocaleTimeString() }}
        </span>
      </div>

      <!-- Compteurs cliquables pour filtrer rapidement -->
      <div class="gsl-stats">
        <button
          type="button"
          class="gsl-stat-badge"
          :class="{ 'gsl-stat-active': selectedLevel === 'all' }"
          title="Afficher toutes les lignes"
          @click="selectedLevel = 'all'"
        >
          <span class="gsl-stat-val">{{ stats.total }}</span>
          <span class="gsl-stat-lbl">Total</span>
        </button>

        <button
          type="button"
          class="gsl-stat-badge gsl-stat-error"
          :class="{ 'gsl-stat-active': selectedLevel === 'error' }"
          title="Filtrer uniquement les erreurs"
          @click="selectedLevel = selectedLevel === 'error' ? 'all' : 'error'"
        >
          <span class="gsl-stat-val">🔴 {{ stats.errors }}</span>
          <span class="gsl-stat-lbl">Erreurs</span>
        </button>

        <button
          type="button"
          class="gsl-stat-badge gsl-stat-warn"
          :class="{ 'gsl-stat-active': selectedLevel === 'warn' }"
          title="Filtrer uniquement les avertissements"
          @click="selectedLevel = selectedLevel === 'warn' ? 'all' : 'warn'"
        >
          <span class="gsl-stat-val">🟡 {{ stats.warnings }}</span>
          <span class="gsl-stat-lbl">Alertes</span>
        </button>

        <button
          type="button"
          class="gsl-stat-badge gsl-stat-player"
          :class="{ 'gsl-stat-active': selectedCategory === 'player' }"
          title="Filtrer les événements joueurs"
          @click="selectedCategory = selectedCategory === 'player' ? 'all' : 'player'"
        >
          <span class="gsl-stat-val">👥 {{ stats.players }}</span>
          <span class="gsl-stat-lbl">Joueurs</span>
        </button>
      </div>
    </header>

    <!-- Barre d'outils 1 : Recherche et Filtres -->
    <div class="gsl-toolbar-search">
      <!-- Recherche textuelle -->
      <div class="gsl-search-wrap">
        <span class="gsl-search-icon">🔍</span>
        <input
          v-model="search"
          type="text"
          class="gsl-search-input"
          placeholder="Rechercher dans les logs (joueur, steam, save, error...)"
        />
        <button
          v-if="search"
          type="button"
          class="gsl-search-clear"
          title="Effacer la recherche"
          @click="search = ''"
        >
          ✕
        </button>
      </div>

      <!-- Filtre de niveau -->
      <div class="gsl-filter-group">
        <span class="gsl-group-label">Niveau :</span>
        <div class="gsl-pills">
          <button
            type="button"
            class="gsl-pill"
            :class="{ active: selectedLevel === 'all' }"
            @click="selectedLevel = 'all'"
          >
            Tous
          </button>
          <button
            type="button"
            class="gsl-pill pill-error"
            :class="{ active: selectedLevel === 'error' }"
            @click="selectedLevel = 'error'"
          >
            Erreurs
          </button>
          <button
            type="button"
            class="gsl-pill pill-warn"
            :class="{ active: selectedLevel === 'warn' }"
            @click="selectedLevel = 'warn'"
          >
            Warnings
          </button>
          <button
            type="button"
            class="gsl-pill pill-success"
            :class="{ active: selectedLevel === 'success' }"
            @click="selectedLevel = 'success'"
          >
            Succès
          </button>
          <button
            type="button"
            class="gsl-pill pill-info"
            :class="{ active: selectedLevel === 'info' }"
            @click="selectedLevel = 'info'"
          >
            Info
          </button>
        </div>
      </div>

      <!-- Filtre par catégorie -->
      <div class="gsl-filter-group">
        <span class="gsl-group-label">Catégorie :</span>
        <div class="gsl-pills">
          <button
            type="button"
            class="gsl-pill"
            :class="{ active: selectedCategory === 'all' }"
            @click="selectedCategory = 'all'"
          >
            Toutes
          </button>
          <button
            type="button"
            class="gsl-pill"
            :class="{ active: selectedCategory === 'player' }"
            @click="selectedCategory = 'player'"
          >
            👥 Joueurs
          </button>
          <button
            type="button"
            class="gsl-pill"
            :class="{ active: selectedCategory === 'save' }"
            @click="selectedCategory = 'save'"
          >
            💾 Saves
          </button>
          <button
            type="button"
            class="gsl-pill"
            :class="{ active: selectedCategory === 'network' }"
            @click="selectedCategory = 'network'"
          >
            🌐 Réseau
          </button>
        </div>
      </div>
    </div>

    <!-- Barre d'outils 2 : Contrôles & Actions de console -->
    <div class="gsl-toolbar-actions">
      <div class="gsl-actions-left">
        <!-- Nombre de lignes -->
        <label class="gsl-ctrl-select">
          <span>Lignes :</span>
          <select v-model.number="lineLimit" class="gsl-select">
            <option :value="100">100</option>
            <option :value="300">300</option>
            <option :value="500">500</option>
            <option :value="1000">1000</option>
          </select>
        </label>

        <!-- Auto-refresh -->
        <label class="gsl-ctrl-select">
          <span>Auto-refresh :</span>
          <select v-model.number="autoRefreshInterval" class="gsl-select">
            <option :value="0">Désactivé</option>
            <option :value="3">3s</option>
            <option :value="5">5s</option>
            <option :value="10">10s</option>
            <option :value="30">30s</option>
          </select>
          <span v-if="autoRefreshInterval > 0" class="gsl-pulse-dot" title="Auto-refresh actif" />
        </label>

        <!-- Taille du texte -->
        <label class="gsl-ctrl-select">
          <span>Texte :</span>
          <select v-model="fontSize" class="gsl-select">
            <option value="sm">Normal (13px)</option>
            <option value="md">Grand (15px)</option>
            <option value="lg">Très grand (18px)</option>
          </select>
        </label>

        <!-- Suivi auto -->
        <button
          type="button"
          class="gsl-btn-toggle"
          :class="{ active: autoScroll }"
          title="Faire défiler automatiquement vers le bas à chaque nouvelle ligne"
          @click="toggleAutoScroll"
        >
          ⬇️ Suivi auto {{ autoScroll ? 'ON' : 'OFF' }}
        </button>

        <!-- Numéros de lignes -->
        <button
          type="button"
          class="gsl-btn-toggle"
          :class="{ active: showLineNumbers }"
          title="Afficher/Masquer les numéros de ligne"
          @click="showLineNumbers = !showLineNumbers"
        >
          # Lignes
        </button>
      </div>

      <div class="gsl-actions-right">
        <!-- Copier -->
        <AppButton
          variant="ghost"
          size="sm"
          :disabled="visibleLines.length === 0"
          title="Copier les logs affichés"
          @click="copyLogs"
        >
          📋 Copier
        </AppButton>

        <!-- Télécharger -->
        <AppButton
          variant="ghost"
          size="sm"
          :disabled="visibleLines.length === 0"
          title="Télécharger les logs au format .log"
          @click="downloadLogs"
        >
          💾 Télécharger
        </AppButton>

        <!-- Vider affichage -->
        <AppButton
          variant="ghost"
          size="sm"
          :disabled="rawLines.length === 0"
          title="Vider la vue locale"
          @click="clearDisplay"
        >
          🧹 Vider
        </AppButton>

        <!-- Plein écran -->
        <AppButton
          variant="ghost"
          size="sm"
          :title="isFullscreen ? 'Quitter le plein écran' : 'Mode plein écran'"
          @click="isFullscreen = !isFullscreen"
        >
          {{ isFullscreen ? '🗗 Réduire' : '⛶ Plein écran' }}
        </AppButton>

        <!-- Rafraîchir -->
        <AppButton
          variant="primary"
          size="sm"
          :disabled="loading"
          @click="fetchLogs(false)"
        >
          <span v-if="loading" class="gsl-spinner">⏳</span>
          <span v-else>🔄</span>
          Rafraîchir
        </AppButton>
      </div>
    </div>

    <!-- Zone Terminale des logs -->
    <div
      ref="terminalRef"
      class="gsl-terminal"
      :class="[`font-${fontSize}`]"
      @scroll="handleScroll"
    >
      <!-- Message d'erreur de chargement -->
      <div v-if="loadError" class="gsl-state-msg gsl-state-error">
        ⚠️ {{ loadError }}
      </div>

      <!-- Aucun log disponible -->
      <div v-else-if="rawLines.length === 0 && !loading" class="gsl-state-msg">
        Aucune ligne de log disponible. Le serveur est peut-être éteint ou vient d'être lancé.
      </div>

      <!-- Aucun résultat avec le filtre actuel -->
      <div v-else-if="visibleLines.length === 0 && rawLines.length > 0" class="gsl-state-msg">
        Aucun log ne correspond aux filtres sélectionnés ({{ selectedLevel !== 'all' ? selectedLevel : '' }} {{ search ? `"${search}"` : '' }}).
      </div>

      <!-- Liste des lignes de log -->
      <ol v-else class="gsl-lines">
        <li
          v-for="line in visibleLines"
          :key="line.id"
          class="gsl-line"
          :class="[`level-${line.level}`, `cat-${line.category}`]"
        >
          <!-- Numéro de ligne -->
          <span v-if="showLineNumbers" class="gsl-line-num">{{ line.id }}</span>

          <!-- Horodatage extrait -->
          <time v-if="line.timestamp" class="gsl-timestamp">{{ line.timestamp }}</time>

          <!-- Badge de niveau -->
          <span class="gsl-level-tag" :class="`tag-${line.level}`">
            {{ line.level.toUpperCase() }}
          </span>

          <!-- Contenu HTML coloré (ANSI + Highlight) -->
          <!-- eslint-disable-next-line vue/no-v-html -->
          <span class="gsl-line-content" v-html="line.html" />
        </li>
      </ol>
    </div>

    <!-- Pied de console : compteur affiché -->
    <footer class="gsl-footer">
      <span>{{ visibleLines.length }} / {{ parsedLines.length }} lignes affichées</span>
      <button
        v-if="!autoScroll"
        type="button"
        class="gsl-scroll-btn"
        @click="scrollToBottom"
      >
        ⬇️ Défiler tout en bas
      </button>
    </footer>
  </div>
</template>

<style scoped>
.gsl-container {
  display: flex;
  flex-direction: column;
  background: var(--bg-card, #13141f);
  border: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  border-radius: var(--radius-md, 8px);
  overflow: hidden;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
  font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  transition: all 0.2s ease;
}

.gsl-fullscreen {
  position: fixed;
  inset: 12px;
  z-index: 1000;
  max-height: calc(100vh - 24px);
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.8);
}

/* En-tête */
.gsl-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 12px 16px;
  background: var(--bg-secondary, #1a1b2b);
  border-bottom: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  flex-wrap: wrap;
}

.gsl-title-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.gsl-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.gsl-icon {
  font-size: 1.25rem;
}

.gsl-title h3 {
  margin: 0;
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--text-primary, #ffffff);
}

.gsl-subtitle {
  font-size: 0.75rem;
  color: var(--text-secondary, #8f95b2);
}

.gsl-stats {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.gsl-stat-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.78rem;
  color: var(--text-primary, #ffffff);
  transition: all 0.15s ease;
}

.gsl-stat-badge:hover {
  background: rgba(255, 255, 255, 0.1);
  transform: translateY(-1px);
}

.gsl-stat-active {
  border-color: var(--accent, #6366f1);
  background: color-mix(in srgb, var(--accent, #6366f1) 20%, transparent);
}

.gsl-stat-val {
  font-weight: 700;
}

.gsl-stat-lbl {
  color: var(--text-secondary, #8f95b2);
  font-size: 0.72rem;
}

/* Barres d'outils */
.gsl-toolbar-search {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 10px 16px;
  background: rgba(0, 0, 0, 0.2);
  border-bottom: 1px solid var(--border, rgba(255, 255, 255, 0.08));
  flex-wrap: wrap;
}

.gsl-search-wrap {
  position: relative;
  flex: 1;
  min-width: 240px;
  display: flex;
  align-items: center;
}

.gsl-search-icon {
  position: absolute;
  left: 10px;
  font-size: 0.85rem;
  pointer-events: none;
  opacity: 0.6;
}

.gsl-search-input {
  width: 100%;
  padding: 6px 30px 6px 32px;
  background: #090a10;
  border: 1px solid var(--border, rgba(255, 255, 255, 0.15));
  border-radius: 6px;
  color: #ffffff;
  font-size: 0.82rem;
}

.gsl-search-input:focus {
  outline: none;
  border-color: var(--accent, #6366f1);
  box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent, #6366f1) 30%, transparent);
}

.gsl-search-clear {
  position: absolute;
  right: 8px;
  background: none;
  border: none;
  color: var(--text-secondary, #8f95b2);
  cursor: pointer;
  font-size: 0.8rem;
  padding: 2px;
}

.gsl-filter-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.gsl-group-label {
  font-size: 0.75rem;
  color: var(--text-secondary, #8f95b2);
  font-weight: 600;
  white-space: nowrap;
}

.gsl-pills {
  display: flex;
  align-items: center;
  gap: 4px;
  background: #090a10;
  padding: 2px;
  border-radius: 6px;
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.gsl-pill {
  background: none;
  border: none;
  padding: 3px 8px;
  border-radius: 4px;
  font-size: 0.73rem;
  font-weight: 500;
  color: var(--text-secondary, #8f95b2);
  cursor: pointer;
  transition: all 0.15s ease;
}

.gsl-pill:hover {
  color: #ffffff;
  background: rgba(255, 255, 255, 0.08);
}

.gsl-pill.active {
  background: var(--accent, #6366f1);
  color: #ffffff;
}

.gsl-pill.pill-error.active {
  background: var(--danger, #ef4444);
}

.gsl-pill.pill-warn.active {
  background: var(--warning, #f59e0b);
  color: #000;
}

.gsl-pill.pill-success.active {
  background: var(--success, #10b981);
}

.gsl-toolbar-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  background: rgba(0, 0, 0, 0.15);
  border-bottom: 1px solid var(--border, rgba(255, 255, 255, 0.08));
  flex-wrap: wrap;
}

.gsl-actions-left,
.gsl-actions-right {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.gsl-ctrl-select {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 0.75rem;
  color: var(--text-secondary, #8f95b2);
  position: relative;
}

.gsl-select {
  background: #090a10;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  color: #ffffff;
  font-size: 0.75rem;
  padding: 3px 6px;
}

.gsl-pulse-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--success, #10b981);
  animation: gsl-pulse 1.5s infinite;
}

@keyframes gsl-pulse {
  0% { transform: scale(0.9); opacity: 0.8; }
  50% { transform: scale(1.4); opacity: 1; box-shadow: 0 0 6px var(--success, #10b981); }
  100% { transform: scale(0.9); opacity: 0.8; }
}

.gsl-btn-toggle {
  background: #090a10;
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 4px;
  color: var(--text-secondary, #8f95b2);
  font-size: 0.75rem;
  padding: 3px 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}

.gsl-btn-toggle.active {
  background: color-mix(in srgb, var(--accent, #6366f1) 25%, transparent);
  border-color: var(--accent, #6366f1);
  color: #ffffff;
  font-weight: 600;
}

/* Zone Terminale */
.gsl-terminal {
  height: 680px;
  min-height: 520px;
  max-height: 85vh;
  overflow-y: auto;
  overflow-x: auto;
  background: #090a10;
  padding: 10px 0;
  font-family: "JetBrains Mono", "Cascadia Code", "Fira Code", Consolas, monospace;
  font-size: 0.95rem;
  line-height: 1.6;
  color: #d1d5db;
}

.gsl-terminal.font-sm {
  font-size: 0.82rem;
  line-height: 1.5;
}

.gsl-terminal.font-md {
  font-size: 0.95rem;
  line-height: 1.6;
}

.gsl-terminal.font-lg {
  font-size: 1.12rem;
  line-height: 1.75;
}

.gsl-fullscreen .gsl-terminal {
  flex: 1;
  height: auto;
  max-height: none;
}

.gsl-state-msg {
  padding: 40px 16px;
  text-align: center;
  color: var(--text-secondary, #8f95b2);
  font-style: italic;
  font-size: 0.95rem;
}

.gsl-state-error {
  color: var(--danger, #ef4444);
  font-weight: 600;
}

.gsl-lines {
  list-style: none;
  margin: 0;
  padding: 0;
}

.gsl-line {
  display: flex;
  align-items: baseline;
  gap: 12px;
  padding: 3px 14px;
  border-left: 3px solid transparent;
  transition: background 0.1s ease;
  white-space: pre-wrap;
  word-break: break-all;
}

.gsl-line:hover {
  background: rgba(255, 255, 255, 0.04);
}

/* Niveaux de ligne */
.gsl-line.level-error {
  border-left-color: var(--danger, #ef4444);
  background: rgba(239, 68, 68, 0.08);
  color: #fca5a5;
}

.gsl-line.level-warn {
  border-left-color: var(--warning, #f59e0b);
  background: rgba(245, 158, 11, 0.06);
  color: #fde68a;
}

.gsl-line.level-success {
  border-left-color: var(--success, #10b981);
  background: rgba(16, 185, 129, 0.05);
  color: #a7f3d0;
}

.gsl-line.level-debug {
  color: #9ca3af;
  opacity: 0.8;
}

.gsl-line-num {
  font-size: 0.78rem;
  color: #4b5563;
  user-select: none;
  min-width: 38px;
  text-align: right;
}

.gsl-timestamp {
  font-size: 0.80rem;
  color: #6b7280;
  user-select: none;
  white-space: nowrap;
}

.gsl-level-tag {
  font-size: 0.70rem;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 3px;
  user-select: none;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.tag-error { background: #ef4444; color: #fff; }
.tag-warn { background: #f59e0b; color: #000; }
.tag-success { background: #10b981; color: #000; }
.tag-info { background: #3b82f6; color: #fff; }
.tag-debug { background: #6b7280; color: #fff; }

.gsl-line-content {
  flex: 1;
}

/* Surbrillance recherche */
:deep(mark.log-match) {
  background: #fbbf24;
  color: #000000;
  padding: 0 2px;
  border-radius: 2px;
  font-weight: 700;
}

/* Couleurs ANSI */
:deep(.ansi-bold) { font-weight: bold; }
:deep(.ansi-dim) { opacity: 0.6; }
:deep(.ansi-italic) { font-style: italic; }
:deep(.ansi-underline) { text-decoration: underline; }
:deep(.ansi-red), :deep(.ansi-bright-red) { color: #f87171; }
:deep(.ansi-green), :deep(.ansi-bright-green) { color: #4ade80; }
:deep(.ansi-yellow), :deep(.ansi-bright-yellow) { color: #fde047; }
:deep(.ansi-blue), :deep(.ansi-bright-blue) { color: #60a5fa; }
:deep(.ansi-magenta), :deep(.ansi-bright-magenta) { color: #f472b6; }
:deep(.ansi-cyan), :deep(.ansi-bright-cyan) { color: #38bdf8; }
:deep(.ansi-white), :deep(.ansi-bright-white) { color: #ffffff; }

/* Pied de page */
.gsl-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 16px;
  background: var(--bg-secondary, #1a1b2b);
  border-top: 1px solid var(--border, rgba(255, 255, 255, 0.1));
  font-size: 0.72rem;
  color: var(--text-secondary, #8f95b2);
}

.gsl-scroll-btn {
  background: var(--accent, #6366f1);
  border: none;
  color: #ffffff;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 0.7rem;
  cursor: pointer;
  font-weight: 600;
}

.gsl-scroll-btn:hover {
  filter: brightness(1.1);
}
</style>
