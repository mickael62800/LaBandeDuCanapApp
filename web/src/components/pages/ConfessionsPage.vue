<script setup lang="ts">
import AppCheckbox from "../atoms/AppCheckbox.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import { computed } from "vue";

import { useConfessions } from "@/composables/useConfessions";
import AppTabs from "../molecules/AppTabs.vue";
import ConfessionsTable from "../organisms/ConfessionsTable.vue";
import ConfessionsReportsTable from "../organisms/ConfessionsReportsTable.vue";
import ConfessionRepliesModal from "../organisms/ConfessionRepliesModal.vue";

const { tab, showDeleted, confessions, reports, loading } = useConfessions();

/// Les compteurs vivent dans le libellé : c'est l'information qu'on vient
/// chercher en regardant un onglet de modération.
const TABS = computed(() => [
  { key: "confessions", label: `Confessions (${confessions.value.length})` },
  { key: "reports", label: `Signalements (${reports.value.length})`, icon: "🚩" },
]);
</script>

<template>
  <AdminPageShell title="Modération des confessions" icon="📝" class="confessions-page">
    <template #lede>
      Confessions anonymes postées via /confess. Seul le owner voit l'auteur réel.
    </template>
    <template #actions>
      <AppCheckbox v-model="showDeleted">Afficher supprimées</AppCheckbox>
    </template>

    <AppTabs
      :model-value="tab"
      :tabs="TABS"
      @update:model-value="tab = $event as typeof tab"
    />

    <div v-if="loading" class="muted">Chargement…</div>
    <ConfessionsTable v-else-if="tab === 'confessions'" />
    <ConfessionsReportsTable v-else-if="tab === 'reports'" />

    <ConfessionRepliesModal />
  </AdminPageShell>
</template>

<style scoped>
.confessions-page { padding: 0; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
</style>
