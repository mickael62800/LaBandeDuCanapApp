<script setup lang="ts">
import { computed, ref, watch } from "vue";
import AppBadge from "@/components/atoms/AppBadge.vue";
import IconButton from "@/components/atoms/IconButton.vue";
import { dockerService, type DockerContainer } from "@/services/dockerService";

type LogLevel = "info" | "warn" | "error";

const props = withDefaults(defineProps<{
  title: string;
  service: string;
  container?: DockerContainer;
  forceLevel?: "all" | LogLevel;
}>(), { forceLevel: "all", container: undefined });

const loading = ref(false);
const loadError = ref("");
const rawLines = ref<string[]>([]);

function lineLevel(line: string): LogLevel {
  if (/\b(error|fatal|panic|failed|failure)\b/i.test(line)) return "error";
  if (/\b(warn|warning)\b/i.test(line)) return "warn";
  return "info";
}

const visibleLines = computed(() => rawLines.value.filter((line) => (
  props.forceLevel === "all" || lineLevel(line) === props.forceLevel
)));

const displayName = computed(() => props.container?.names[0]?.replace(/^\//, "") ?? props.service);

async function fetchLogs() {
  if (!props.container) {
    rawLines.value = [];
    return;
  }
  loading.value = true;
  loadError.value = "";
  try {
    const response = await dockerService.containerLogs(props.container.id, 500, true);
    rawLines.value = response.logs
      .split(/\r?\n/)
      .map(line => line.trimEnd())
      .filter(Boolean)
      .reverse();
  } catch (error) {
    loadError.value = error instanceof Error ? error.message : "Logs indisponibles";
  } finally {
    loading.value = false;
  }
}

watch(() => props.container?.id, fetchLogs, { immediate: true });
</script>

<template>
  <section class="container-logs">
    <header class="column-head">
      <div class="service-copy">
        <div class="title-line">
          <h3>{{ title }}</h3>
          <AppBadge
            v-if="container"
            :label="container.state"
            :variant="container.state === 'running' ? 'success' : 'warning'"
          />
        </div>
        <span>{{ displayName }}</span>
      </div>
      <IconButton label="Actualiser les logs" size="sm" :disabled="loading || !container" @click="fetchLogs">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 11a8.1 8.1 0 0 0-15.5-2M4 4v5h5" />
          <path d="M4 13a8.1 8.1 0 0 0 15.5 2M20 20v-5h-5" />
        </svg>
      </IconButton>
    </header>

    <div v-if="!container" class="column-state">Service Docker introuvable.</div>
    <div v-else-if="loading" class="column-state">Chargement…</div>
    <div v-else-if="loadError" class="column-state error">{{ loadError }}</div>
    <div v-else-if="visibleLines.length === 0" class="column-state">Aucune ligne pour ce niveau.</div>
    <ol v-else class="lines">
      <li v-for="(line, index) in visibleLines" :key="`${index}-${line}`" :class="`line-${lineLevel(line)}`">
        <AppBadge :label="lineLevel(line)" :variant="lineLevel(line)" />
        <code>{{ line }}</code>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.container-logs {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 280px);
  min-height: 400px;
  overflow: hidden;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}

.column-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
}

.service-copy { min-width: 0; }
.title-line { display: flex; align-items: center; gap: 8px; }
.title-line h3 { margin: 0; font-size: 13px; text-transform: uppercase; }
.service-copy > span {
  display: block;
  margin-top: 3px;
  overflow: hidden;
  color: var(--text-secondary);
  font: 10px "JetBrains Mono", monospace;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.column-head svg { width: 14px; height: 14px; }

.lines {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 0;
  overflow-y: auto;
  list-style: none;
}

.lines li {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  align-items: start;
  gap: 8px;
  padding: 8px 10px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent);
}

.lines li.line-warn { border-left: 3px solid var(--warning); }
.lines li.line-error { border-left: 3px solid var(--danger); }

.lines code {
  color: var(--text-primary);
  font: 11px/1.45 "JetBrains Mono", monospace;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.column-state {
  display: grid;
  flex: 1;
  place-items: center;
  padding: 24px;
  color: var(--text-secondary);
  font-size: 12px;
  text-align: center;
}
.column-state.error { color: var(--danger); }
</style>
