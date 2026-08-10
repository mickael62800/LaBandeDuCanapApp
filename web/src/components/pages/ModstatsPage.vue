<script setup lang="ts">
import { computed, ref } from "vue";
import ModstatsChartsGrid from "../organisms/ModstatsChartsGrid.vue";

const days = ref(30);
const periods = computed(() => [7, 14, 30, 90]);
const refreshing = ref(false);

const gridRef = ref<InstanceType<typeof ModstatsChartsGrid> | null>(null);

async function handleRefresh() {
  if (refreshing.value) return;
  refreshing.value = true;
  try {
    await gridRef.value?.refresh();
  } finally {
    refreshing.value = false;
  }
}
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `StatsHubPage`. -->
  <div class="dashboard">
    <div class="tab-toolbar">
      <div class="header-actions">
        <div class="period-selector">
          <button
            v-for="p in periods"
            :key="p"
            :class="['period-btn', { active: days === p }]"
            @click="days = p"
          >
            {{ p }}j
          </button>
        </div>
        <button
          class="refresh-btn"
          :disabled="refreshing"
          :title="refreshing ? 'Actualisation en cours…' : 'Actualiser les données'"
          @click="handleRefresh"
        >
          <svg
            :class="['refresh-icon', { spinning: refreshing }]"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
            <path d="M21 3v5h-5" />
            <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
            <path d="M3 21v-5h5" />
          </svg>
          <span>Actualiser</span>
        </button>
      </div>
    </div>

    <ModstatsChartsGrid ref="gridRef" :days="days" />
  </div>
</template>

<style scoped>
/* Barre d'outils de l'onglet. Le titre degrade et sa bordure sont partis
   dans `AdminPageShell`, porte par le hub — ce bloc etait recopie a
   l'identique depuis `StatsPage`. */
.tab-toolbar {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 20px;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

/* Période selector — segmented control polished */
.period-selector {
  display: flex;
  gap: 2px;
  background-color: color-mix(in srgb, var(--bg-card) 80%, transparent);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 3px;
  position: relative;
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.18),
    0 1px 0 color-mix(in srgb, white 6%, transparent);
}
.period-btn {
  position: relative;
  padding: 6px 14px;
  border-radius: var(--radius-sm);
  background: none;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background 0.25s ease, box-shadow 0.25s ease;
}
.period-btn::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 3px;
  width: 0;
  height: 2px;
  border-radius: var(--radius-xs);
  background: var(--accent);
  transform: translateX(-50%);
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.period-btn:hover:not(.active) {
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--accent) 8%, transparent);
}
.period-btn:hover:not(.active)::after { width: 60%; }
.period-btn.active {
  background: linear-gradient(135deg,
    var(--accent),
    color-mix(in srgb, var(--accent) 75%, var(--accent-alt, #a855f7)));
  color: white;
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 35%, transparent),
    inset 0 -1px 0 color-mix(in srgb, black 15%, transparent),
    0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
  text-shadow: 0 1px 1px rgba(0, 0, 0, 0.12);
}
.period-btn:active {
  transform: scale(0.96);
  transition-duration: 0.08s;
}

/* Refresh button polished */
.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 7px 14px;
  border-radius: var(--radius-md);
  background:
    linear-gradient(180deg,
      color-mix(in srgb, white 4%, var(--bg-card)),
      var(--bg-card));
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background 0.25s ease, border-color 0.2s ease, box-shadow 0.25s ease;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}
.refresh-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--accent) 10%, var(--bg-card)),
    color-mix(in srgb, var(--accent) 6%, var(--bg-card)));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 10%, transparent),
    0 4px 12px color-mix(in srgb, var(--accent) 18%, transparent);
}
.refresh-btn:hover:not(:disabled) .refresh-icon:not(.spinning) {
  transform: rotate(180deg);
}
.refresh-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
}
.refresh-btn:disabled { opacity: 0.6; cursor: not-allowed; }

.refresh-icon {
  width: 14px;
  height: 14px;
  transition: transform 0.45s cubic-bezier(0.4, 0, 0.2, 1);
}
.refresh-icon.spinning { animation: spin 0.9s linear infinite; }
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@media (prefers-reduced-motion: reduce) {
  /* L'animation du titre est desormais geree par `AdminPageShell`. */
  .period-btn,
  .period-btn:hover,
  .period-btn:active,
  .refresh-btn,
  .refresh-btn:hover,
  .refresh-btn:active { transform: none; }
  .refresh-icon { transition: none !important; }
  .period-btn::after { transition: none !important; }
}
</style>
