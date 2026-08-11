<script setup lang="ts">
import AppButton from "@/components/atoms/AppButton.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";

defineProps<{
  search: string;
  type: string;
  moderator: string;
  status: "all" | "detection" | "action";
  dateFrom: string;
  dateTo: string;
  hideDetections: boolean;
  typeOptions: Array<{ value: string; label: string }>;
  moderatorOptions: Array<{ value: string; label: string }>;
  statusOptions: Array<{ value: string; label: string }>;
  hasActiveFilters: boolean;
  selectedGuildId: string | null;
  purging: boolean;
  bulkMenuOpen: boolean;
}>();

const emit = defineEmits<{
  "update:search": [value: string];
  "update:type": [value: string];
  "update:moderator": [value: string];
  "update:status": [value: "all" | "detection" | "action"];
  "update:dateFrom": [value: string];
  "update:dateTo": [value: string];
  "update:hideDetections": [value: boolean];
  "update:bulkMenuOpen": [value: boolean];
  reset: [];
  purge: [];
  create: [];
}>();
</script>

<template>
  <div class="card journal-toolbar">
    <div class="filters-grid">
      <div class="filter-field filter-search">
        <label>Recherche</label>
        <AppInput
          :model-value="search"
          placeholder="Utilisateur, ID, raison, serveur…"
          @update:model-value="emit('update:search', String($event))"
        />
      </div>
      <div class="filter-field">
        <label>Type</label>
        <AppSelect
          :model-value="type"
          :options="typeOptions"
          @update:model-value="emit('update:type', $event)"
        />
      </div>
      <div class="filter-field">
        <label>Moderateur</label>
        <AppSelect
          :model-value="moderator"
          :options="moderatorOptions"
          @update:model-value="emit('update:moderator', $event)"
        />
      </div>
      <div class="filter-field">
        <label>Statut</label>
        <AppSelect
          :model-value="status"
          :options="statusOptions"
          @update:model-value="emit('update:status', $event as 'all' | 'detection' | 'action')"
        />
      </div>
      <div class="filter-field">
        <label>Du</label>
        <input
          :value="dateFrom"
          type="date"
          class="date-input"
          @input="emit('update:dateFrom', ($event.target as HTMLInputElement).value)"
        />
      </div>
      <div class="filter-field">
        <label>Au</label>
        <input
          :value="dateTo"
          type="date"
          class="date-input"
          @input="emit('update:dateTo', ($event.target as HTMLInputElement).value)"
        />
      </div>
    </div>

    <div class="toolbar-right">
      <label class="toggle-filter">
        <span>Masquer les détections AutoMod</span>
        <span class="switch">
          <input
            :checked="hideDetections"
            type="checkbox"
            @change="emit('update:hideDetections', ($event.target as HTMLInputElement).checked)"
          />
          <span class="slider" aria-hidden="true"></span>
        </span>
      </label>
      <button
        v-if="hasActiveFilters"
        class="reset-btn"
        title="Reinitialiser les filtres"
        @click="emit('reset')"
      >
        Reinitialiser
      </button>
      <div class="bulk-menu-wrap" @click.stop>
        <button
          class="bulk-menu-btn"
          :disabled="!selectedGuildId"
          :title="selectedGuildId ? 'Actions de masse (owner uniquement)' : 'Selectionnez un serveur'"
          @click="emit('update:bulkMenuOpen', !bulkMenuOpen)"
        >
          ⋯ Actions de masse ▾
        </button>
        <div v-if="bulkMenuOpen" class="bulk-menu">
          <button
            class="bulk-item danger"
            :disabled="purging"
            title="Vide le journal de la base de données. NE débannit PAS et NE retire PAS les mutes sur Discord."
            @click="emit('update:bulkMenuOpen', false); emit('purge')"
          >
            🗑 {{ purging ? "Suppression…" : "Vider le journal (DB seule)" }}
          </button>
        </div>
      </div>
      <AppButton variant="primary" @click="emit('create')">+ Nouvelle action</AppButton>
    </div>
  </div>
</template>
