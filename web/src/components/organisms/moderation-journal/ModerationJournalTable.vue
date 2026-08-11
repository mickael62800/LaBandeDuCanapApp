<script setup lang="ts">
import AppBadge from "@/components/atoms/AppBadge.vue";
import ErrorState from "@/components/atoms/ErrorState.vue";
import LoadingState from "@/components/atoms/LoadingState.vue";
import DataTable from "@/components/organisms/DataTable.vue";
import { useFormatDate } from "@/composables/useFormatDate";
import type { Infraction, TableColumn } from "@/types";
import { infractionTypeVariant } from "@/utils/variants";

defineProps<{
  rows: Infraction[];
  total: number;
  loading: boolean;
  error: string | null;
  applying: boolean;
  deleting: boolean;
}>();

const emit = defineEmits<{
  retry: [];
  apply: [row: Record<string, unknown>];
  remove: [row: Record<string, unknown>];
  "open-notes": [userId: string];
}>();

const { formatShortDateTime: fmt } = useFormatDate();

const columns: TableColumn[] = [
  { key: "username", label: "Utilisateur" },
  { key: "infraction_type", label: "Type" },
  { key: "source", label: "Choix" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
  { key: "actions", label: "" },
];

function isDetection(type: string | null | undefined): boolean {
  const normalized = String(type ?? "").toLowerCase();
  return normalized === "" || normalized === "none" || normalized === "detection";
}

function infractionTypeLabel(type: string | null | undefined): string {
  return isDetection(type) ? "Detection" : String(type);
}
</script>

<template>
  <div class="result-count">
    <strong>{{ rows.length }}</strong>
    infraction{{ rows.length > 1 ? "s" : "" }}
    <span v-if="rows.length !== total" class="result-total">sur {{ total }}</span>
  </div>

  <ErrorState v-if="error" :message="error" :retryable="true" @retry="emit('retry')" />
  <LoadingState v-else-if="loading" />

  <DataTable
    v-else
    :columns="columns"
    :rows="(rows as unknown as Record<string, unknown>[] )"
    empty-message="Aucune infraction ne correspond aux filtres"
  >
    <template #cell-username="{ row }">
      <div class="user-cell">
        <strong v-if="row.display_name" class="display-name">{{ row.display_name }}</strong>
        <span class="username">@{{ row.username }}</span>
        <span class="user-id">{{ row.user_id }}</span>
      </div>
    </template>
    <template #cell-infraction_type="{ value }">
      <AppBadge
        :label="infractionTypeLabel(String(value))"
        :variant="isDetection(String(value)) ? 'default' : infractionTypeVariant(String(value))"
      />
    </template>
    <template #cell-source="{ row, value }">
      <span
        v-if="value === 'detection' && !isDetection(String(row.infraction_type))"
        class="source-chip proposal"
        title="Detection AutoMod : proposition, pas encore appliquee"
      >Proposition</span>
      <span
        v-else-if="value === 'action'"
        class="source-chip applied"
        title="Sanction effectivement appliquee par un moderateur ou un bot"
      >Applique</span>
      <span v-else class="source-chip neutral">—</span>
    </template>
    <template #cell-created_at="{ value }">
      <span class="mono">{{ fmt(String(value)) }}</span>
    </template>
    <template #cell-actions="{ row }">
      <div class="action-buttons">
        <button
          class="notes-btn"
          title="Voir / ajouter notes et preuves pour cet utilisateur"
          @click.stop="emit('open-notes', String(row.user_id))"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
          <span>📎</span>
        </button>
        <button
          v-if="row.source === 'detection' && !isDetection(String(row.infraction_type))"
          class="apply-btn"
          :disabled="applying"
          title="Appliquer cette proposition (ban/mute/warn)"
          @click.stop="emit('apply', row)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12" />
          </svg>
          <span>Appliquer</span>
        </button>
        <button
          class="cancel-btn"
          :disabled="deleting"
          title="Annuler cette entree (si ban applique, unban Discord inclus)"
          @click.stop="emit('remove', row)"
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 6h18" />
            <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
            <path d="M10 11v6" />
            <path d="M14 11v6" />
          </svg>
          <span>Annuler</span>
        </button>
      </div>
    </template>
  </DataTable>
</template>
