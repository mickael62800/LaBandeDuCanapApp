<script setup lang="ts">
import { ref } from "vue";
import { useDiscordRoles } from "../../composables/useDiscordRoles";
import type { DiscordRole } from "../../types";
import AppInput from "../atoms/AppInput.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import DiscordRolesGrid from "../organisms/DiscordRolesGrid.vue";
import DiscordRoleCreateModal from "../organisms/DiscordRoleCreateModal.vue";
import DiscordRoleEditModal from "../organisms/DiscordRoleEditModal.vue";

const {
  filteredRoles,
  totalRoles,
  loading,
  error,
  search,
  fetchRoles,
} = useDiscordRoles();

const showCreateModal = ref(false);
const editingRole = ref<DiscordRole | null>(null);

function openEdit(role: DiscordRole) {
  editingRole.value = role;
}
function closeEdit() {
  editingRole.value = null;
}
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `RolesHubPage`.
       Le lien croise vers les panneaux a disparu : l'onglet voisin fait
       exactement ce qu'il faisait, en mieux. -->
  <div class="discord-roles">
    <div class="toolbar">
      <AppInput v-model="search" placeholder="Rechercher un role..." />
      <span class="role-count">{{ totalRoles }} roles</span>
      <button class="btn-create" @click="showCreateModal = true">+ Creer un role</button>
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchRoles" />
    <LoadingState v-else-if="loading" />
    <EmptyState v-else-if="filteredRoles.length === 0" message="Aucun role trouve" />
    <DiscordRolesGrid v-else @edit="openEdit" />

    <DiscordRoleCreateModal :visible="showCreateModal" @close="showCreateModal = false" />
    <DiscordRoleEditModal :target="editingRole" @close="closeEdit" />
  </div>
</template>

<style scoped>
.discord-roles h1 { margin: 0; }

.header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.cross-link {
  margin-left: auto;
  font-size: 13px;
  font-weight: 600;
  color: var(--accent);
  text-decoration: none;
  padding: 8px 16px;
  border: 1px solid var(--accent);
  border-radius: var(--radius-md);
  white-space: nowrap;
  transition: all var(--transition-fast);
}
.cross-link:hover { background: var(--accent); color: white; }

.role-count {
  font-size: 13px;
  color: var(--text-secondary);
  background: var(--bg-card);
  padding: 4px 10px;
  border-radius: var(--radius-lg);
}

.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}
.toolbar :deep(input) { max-width: 360px; }

.btn-create {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: var(--radius-md);
  padding: 10px 20px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: opacity var(--transition-base);
}
.btn-create:hover { opacity: 0.85; }

@media (max-width: 768px) {
  .header { flex-wrap: wrap; gap: 8px; }
  .cross-link { margin-left: 0; flex: 1; text-align: center; }
  .toolbar { flex-direction: column; gap: 8px; }
  .toolbar :deep(input) { max-width: 100%; }
  .btn-create { width: 100%; }
}
</style>
