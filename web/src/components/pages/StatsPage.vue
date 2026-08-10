<script setup lang="ts">
import { ref, computed } from "vue";
import DashboardChartsSection from "../organisms/DashboardChartsSection.vue";
import { registerChartJs } from "@/utils/chartjs";
import { analyticsService } from "@/services/analyticsService";
import { useGuildSelectorStore } from "@/stores/guildSelectorStore";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { errMsg } from "@/utils/errMsg";

registerChartJs();

const { success, error: showError, warning } = useToast();
const { confirm } = useConfirm();

const days = ref(30);

const chartsRef = ref<InstanceType<typeof DashboardChartsSection> | null>(null);

const refreshing = ref(false);
const resetting = ref(false);
const guildStore = useGuildSelectorStore();

async function handleRefresh() {
  refreshing.value = true;
  try {
    await chartsRef.value?.refresh();
  } finally {
    refreshing.value = false;
  }
}

async function handleReset() {
  const gid = guildStore.selectedGuildId;
  if (!gid) {
    warning("Sélectionne d'abord un serveur.");
    return;
  }
  const ok = await confirm({
    title: "Vider les statistiques d'activité",
    message:
      "Vider toutes les statistiques d'activité (heatmap, pics horaires, etc.) pour ce serveur ? " +
      "Les infractions et logs d'audit seront CONSERVÉS — seuls les compteurs d'activité (hourly_activity / daily_activity) seront remis à zéro. " +
      "Action irréversible.",
  });
  if (!ok) return;
  resetting.value = true;
  try {
    const res = await analyticsService.reset(gid);
    success(`Analytics remises à zéro : ${res.deleted_rows} lignes supprimées.`);
    await chartsRef.value?.refresh();
  } catch (e) {
    console.error("Reset analytics echoue", e);
    showError(`Échec du reset : ${errMsg(e)}`);
  } finally {
    resetting.value = false;
  }
}

const periods = computed(() => [7, 14, 30, 90]);
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `StatsHubPage`.
       Le titre degrade et sa bordure vivaient ici — c'est d'ailleurs d'ici
       qu'`AdminPageShell` les a repris. Le shell est desormais la reference,
       cette page n'en garde que sa barre d'outils. -->
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
          :title="refreshing ? 'Actualisation en cours…' : 'Actualiser les donnees'"
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
        <button
          class="reset-btn"
          :disabled="resetting"
          :title="resetting ? 'Reset en cours…' : 'Vider les compteurs d\'activite (irreversible)'"
          @click="handleReset"
        >
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            width="14"
            height="14"
          >
            <path d="M3 6h18" />
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
            <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
          </svg>
          <span>{{ resetting ? "Reset…" : "Remettre a zero" }}</span>
        </button>
      </div>
    </div>

    <DashboardChartsSection ref="chartsRef" :days="days" />
  </div>
</template>

<style scoped>
/* Barre d'outils de l'onglet (periode, actualisation). Le titre et sa
   bordure degradee sont partis dans `AdminPageShell`, porte par le hub. */
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

/* ── Période selector — segmented control style cosy ─────── */
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
  transition: color 0.2s ease,
    background 0.25s ease,
    box-shadow 0.25s ease;
}

/* Indicateur subtil sous chaque bouton inactif au hover (souligné court). */
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
.period-btn:hover:not(.active)::after {
  width: 60%;
}

.period-btn.active {
  /* Gradient discret + double shadow (interne lumineuse + externe douce)
     pour donner un effet "embouti / glossy" sans être agressif. */
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
  /* Petit enfoncement tactile au clic. */
  transform: scale(0.96);
  transition-duration: 0.08s;
}

/* ── Refresh button — propre, tactile, accent au hover ─────── */
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
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  /* Inner highlight subtil pour la sensation "embouti". */
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

/* L'icône fait un demi-tour d'aperçu au hover (preview du refresh). */
.refresh-btn:hover:not(:disabled) .refresh-icon:not(.spinning) {
  transform: rotate(180deg);
}
.refresh-icon {
  transition: transform 0.45s cubic-bezier(0.4, 0, 0.2, 1);
}

.refresh-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* ── Reset button — destructif (rouge), confirmation requise ─────── */
.reset-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 7px 14px;
  border-radius: var(--radius-md);
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--danger) 8%, var(--bg-card)),
    var(--bg-card));
  border: 1px solid color-mix(in srgb, var(--danger) 35%, var(--border));
  color: color-mix(in srgb, var(--danger) 80%, var(--text-secondary));
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background 0.25s ease, border-color 0.2s ease, box-shadow 0.25s ease;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.reset-btn:hover:not(:disabled) {
  color: white;
  border-color: var(--danger);
  background: linear-gradient(180deg, var(--danger), var(--danger));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 25%, transparent),
    0 4px 12px color-mix(in srgb, var(--danger) 30%, transparent);
}

.reset-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
}

.reset-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
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

.refresh-icon {
  width: 14px;
  height: 14px;
}

.refresh-icon.spinning {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
