<script setup lang="ts">
import { computed } from "vue";
import type { SystemInfo } from "@/services/systemService";

const props = defineProps<{ info: SystemInfo }>();

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}j ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function diskBarColor(pct: number): string {
  if (pct >= 90) return "var(--danger)";
  if (pct >= 75) return "var(--warning, #e67e22)";
  return "var(--success, #2ecc71)";
}

const ramPct = computed(() =>
  props.info.host.mem_total_mb > 0
    ? (props.info.host.mem_used_mb / props.info.host.mem_total_mb) * 100
    : 0,
);
const cpuPct = computed(() => props.info.host.cpu_percent ?? 0);

/**
 * Rend un débit lisible : 2 300 000 o/s ne se lit pas, « 2,3 Mo/s » oui.
 */
function debit(octetsParSeconde: number): string {
  const v = Number(octetsParSeconde) || 0;
  if (v < 1024) return `${v} o/s`;
  if (v < 1024 * 1024) return `${(v / 1024).toFixed(1)} Ko/s`;
  return `${(v / (1024 * 1024)).toFixed(2)} Mo/s`;
}
</script>

<template>
  <section class="dash-section">
    <h2 class="section-title">Ressources host</h2>
    <div class="metrics-grid">
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-label">CPU host</span>
          <span class="metric-value">{{ cpuPct.toFixed(1) }}%</span>
        </div>
        <div class="bar">
          <div class="bar-fill" :style="{ width: `${Math.min(cpuPct, 100)}%`, background: diskBarColor(cpuPct) }"></div>
        </div>
        <!-- « Processeurs logiques » et non « cœurs » : le noyau compte les
             threads. Un 8 cœurs / 16 threads affiche 16, et le mot « cœurs »
             ferait douter d'une mesure pourtant juste. -->
        <div class="metric-sub">{{ info.host.cpu_cores }} processeurs logiques</div>
      </div>
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-label">RAM host</span>
          <span class="metric-value">{{ ramPct.toFixed(1) }}%</span>
        </div>
        <div class="bar">
          <div class="bar-fill" :style="{ width: `${Math.min(ramPct, 100)}%`, background: diskBarColor(ramPct) }"></div>
        </div>
        <div class="metric-sub">
          {{ info.host.mem_used_mb.toLocaleString() }} / {{ info.host.mem_total_mb.toLocaleString() }} MB
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-label">Réseau host</span>
          <span class="metric-value">{{ debit(info.host.net_rx_bytes_per_sec + info.host.net_tx_bytes_per_sec) }}</span>
        </div>
        <!-- Pas de barre : un débit n'a pas de plafond connu, et en inventer
             un ferait lire « 80 % » là où il n'y a rien à comparer. -->
        <div class="metric-sub">
          ↓ {{ debit(info.host.net_rx_bytes_per_sec) }} &nbsp;·&nbsp;
          ↑ {{ debit(info.host.net_tx_bytes_per_sec) }}
        </div>
      </div>
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-label">Charge système</span>
          <span
            class="metric-value"
            :style="{ color: info.host.load_1m > info.host.cpu_cores ? 'var(--danger)' : undefined }"
          >
            {{ info.host.load_1m.toFixed(2) }}
          </span>
        </div>
        <!-- Comparée aux cœurs : au-delà, des tâches attendent leur tour, et
             c'est là que naissent les lags — un processus qui attend ne
             consomme pas de CPU, donc le pourcentage ne le montre pas. -->
        <div class="metric-sub">
          5 min : {{ info.host.load_5m.toFixed(2) }} · {{ info.host.cpu_cores }} processeurs
          logiques
          <template v-if="info.host.load_1m > info.host.cpu_cores"> — saturée</template>
        </div>
      </div>

      <!-- Connectivité vers l'extérieur. Le témoin (DNS public) sert à situer
           la panne : Discord injoignable alors que le témoin répond désigne
           Discord, pas la machine. -->
      <div v-for="probe in info.host.internet ?? []" :key="probe.target" class="metric-card">
        <div class="metric-header">
          <span class="metric-label">{{ probe.label }}</span>
          <span class="metric-value" :style="{ color: probe.reachable ? undefined : 'var(--danger)' }">
            {{ probe.reachable ? `${probe.latency_ms} ms` : "injoignable" }}
          </span>
        </div>
        <div class="metric-sub">{{ probe.target }}</div>
      </div>

      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-label">CPU process API</span>
          <span class="metric-value">{{ info.process.cpu_percent.toFixed(1) }}%</span>
        </div>
        <div class="bar">
          <div class="bar-fill" :style="{ width: `${Math.min(info.process.cpu_percent, 100)}%`, background: 'var(--accent)' }"></div>
        </div>
        <div class="metric-sub">RAM process : {{ info.process.mem_used_mb }} MB</div>
      </div>
      <div class="metric-card">
        <div class="metric-header">
          <span class="metric-label">Redis</span>
          <span class="metric-value">{{ info.redis.used_memory_mb }} MB</span>
        </div>
        <div class="metric-sub">
          {{ info.redis.connected_clients }} clients · {{ info.redis.total_keys.toLocaleString() }} clés
          · uptime {{ formatUptime(info.redis.uptime_seconds) }}
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.dash-section { margin-bottom: 24px; }
.section-title {
  position: relative;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0 0 14px 0;
  padding: 0 0 8px 14px;
  border-bottom: 1px solid var(--border);
}
.section-title::before {
  content: "";
  position: absolute;
  left: 0;
  top: 2px;
  bottom: 14px;
  width: 3px;
  border-radius: var(--radius-xs);
  background: linear-gradient(to bottom, var(--accent), color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}

.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 14px;
}

.metric-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px 16px;
}
.metric-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}
.metric-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
.metric-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
}
.metric-sub {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 6px;
}

.bar {
  height: 8px;
  background: var(--bg-secondary);
  border-radius: var(--radius-sm);
  overflow: hidden;
  position: relative;
}
.bar-fill {
  height: 100%;
  border-radius: var(--radius-sm);
  transition: width 0.4s ease, background 0.3s ease;
}
</style>
