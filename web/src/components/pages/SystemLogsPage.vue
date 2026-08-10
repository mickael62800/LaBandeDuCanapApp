<script setup lang="ts">
import { ref } from "vue";
import LogsColumn from "../organisms/LogsColumn.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";

// Filtre de niveau global propage aux 4 colonnes. Chaque colonne garde
// la possibilite d'override en local (le select de la colonne reste).
const globalLevel = ref<"all" | "info" | "warn" | "error">("all");
</script>

<template>
  <AdminPageShell title="Logs techniques" icon="⚙️" width="wide" class="system-logs">
    <template #lede>
      Logs des bots Discord, workers, requêtes API et WebSocket — affichés
      en parallèle pour faciliter le diagnostic d'incidents qui touchent
      plusieurs catégories en même temps.
    </template>
    <template #actions>
      <div class="global-filter">
        <label for="lvl-global">Niveau global</label>
        <AppSelect id="lvl-global" v-model="globalLevel" class="lvl-select">
          <option value="all">Tous</option>
          <option value="info">Info</option>
          <option value="warn">Warn</option>
          <option value="error">Error</option>
        </AppSelect>
      </div>
    </template>

    <div class="system-grid">
      <LogsColumn title="Bots" category="bot" :force-level="globalLevel" />
      <LogsColumn title="Workers" category="worker" :force-level="globalLevel" />
      <LogsColumn title="API" category="api" :force-level="globalLevel" />
      <LogsColumn title="WebSocket" category="websocket" :force-level="globalLevel" />
    </div>
  </AdminPageShell>
</template>

<style scoped>
.system-logs { padding: 0; }
.global-filter {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}
.lvl-select {
  height: 32px;
  padding: 4px 10px;
  font-size: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  min-width: 140px;
}

.system-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 16px;
  align-items: start;
}
@media (max-width: 1400px) {
  .system-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 800px) {
  .system-grid { grid-template-columns: 1fr; }
}
</style>
