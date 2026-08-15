<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { systemService, type SystemInfo } from "@/services/systemService";
import { useToast } from "@/composables/useToast";
import DockerAdminSection from "@/components/organisms/DockerAdminSection.vue";
import DiskTrendChart from "@/components/organisms/DiskTrendChart.vue";
import ServerHealthBanner from "@/components/organisms/ServerHealthBanner.vue";
import ServerHealthResources from "@/components/organisms/ServerHealthResources.vue";
import ServerHealthDisks from "@/components/organisms/ServerHealthDisks.vue";
import ServerHealthServices from "@/components/organisms/ServerHealthServices.vue";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import DashboardHero from "@/components/organisms/DashboardHero.vue";

const { error: showError } = useToast();

const info = ref<SystemInfo | null>(null);
const loading = ref(true);
const refreshing = ref(false);
const autoRefresh = ref(true);
let pollHandle: number | null = null;

async function fetchInfo() {
  refreshing.value = true;
  try {
    info.value = await systemService.getInfo();
  } catch (e) {
    console.error(e);
    showError("Erreur chargement système.");
  } finally {
    loading.value = false;
    refreshing.value = false;
  }
}

function startPolling() {
  if (pollHandle !== null) return;
  pollHandle = window.setInterval(fetchInfo, 120_000);
}
function stopPolling() {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}
function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
  if (autoRefresh.value) startPolling();
  else stopPolling();
}

onMounted(() => {
  fetchInfo();
  if (autoRefresh.value) startPolling();
});
onUnmounted(stopPolling);
</script>

<template>
  <AdminPageShell class="dashboard">
    <DashboardHero
      title="Exploitation"
      subtitle="Ressources, disques, services et conteneurs de la machine hôte — celle qui héberge Sentinel, Nexus et Atrium."
      logo="/exploration_logo.png"
      universe="ops"
    >
      <template #actions>
        <button
          class="auto-toggle"
          :class="{ active: autoRefresh }"
          :title="autoRefresh ? 'Auto-refresh actif (2 min)' : 'Auto-refresh désactivé'"
          @click="toggleAutoRefresh"
        >
          <span class="dot" :class="{ pulse: autoRefresh }"></span>
          {{ autoRefresh ? "Live (2 min)" : "Pause" }}
        </button>
        <button
          class="refresh-btn"
          :disabled="refreshing"
          @click="fetchInfo"
        >
          <svg
            :class="['refresh-icon', { spinning: refreshing }]"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            width="14"
            height="14"
          >
            <polyline points="23 4 23 10 17 10"></polyline>
            <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10"></path>
          </svg>
          Actualiser
        </button>
      </template>
    </DashboardHero>

    <div v-if="loading" class="loading">Chargement…</div>

    <template v-else-if="info">
      <ServerHealthBanner :info="info" />
      <ServerHealthResources :info="info" />
      <ServerHealthDisks :info="info" />
      <ServerHealthServices :info="info" />

      <!-- Disque tendance 7j (depuis cron host) -->
      <DiskTrendChart />

      <!-- Administration Docker (overview, conteneurs, images, volumes, networks, prune) -->
      <DockerAdminSection />
    </template>
  </AdminPageShell>
</template>

<style scoped>
/* L'en-tete (titre + bordure degradee) vient d'`AdminPageShell`. Elle etait
   recopiee ici a l'identique, gradient compris. */

.auto-toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 14px;
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.auto-toggle.active {
  color: var(--success, var(--success));
  border-color: color-mix(in srgb, var(--success, var(--success)) 50%, var(--border));
}

.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-secondary);
}
.dot.pulse {
  background: var(--success, var(--success));
  animation: dot-pulse 1.2s ease-in-out infinite;
}
@keyframes dot-pulse {
  0%, 100% { opacity: 0.5; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.3); }
}

.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.refresh-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
}
.refresh-icon.spinning { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

.loading { text-align: center; padding: 40px; color: var(--text-secondary); }
</style>
