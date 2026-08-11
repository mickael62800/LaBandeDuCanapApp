<script setup lang="ts">
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { onMounted, watch, ref, computed } from "vue";
import { useMembers } from "../../composables/useMembers";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { usePagination } from "../../composables/usePagination";
import ErrorState from "../atoms/ErrorState.vue";
import AppBadge from "../atoms/AppBadge.vue";
import MemberStatusBadge from "../atoms/MemberStatusBadge.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import MemberDetailDrawer from "../organisms/MemberDetailDrawer.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import { formatShortMonthDate as formatDate } from "../../composables/useFormatDate";

const { selectedGuildId } = useGuildSelector();

const {
  filteredMembers,
  loading,
  error,
  search,
  sortBy,
  selectedMember,
  isWatched,
  fetchMembers,
  selectMember,
  closeMember,
} = useMembers();

const watchFilter = ref<"all" | "watched" | "unwatched">("all");
// Filtre de presence : par defaut on n'affiche que les membres PRESENTS
// (left_at null). Les membres partis (left_at renseigne) sont masques sauf
// si on choisit "Partis" ou "Tous".
const presenceFilter = ref<"present" | "left" | "all">("present");

const tabFilteredMembers = computed(() => {
  let list = filteredMembers.value.filter((m) => !m.is_bot);
  if (presenceFilter.value === "present") list = list.filter((m) => !m.left_at);
  else if (presenceFilter.value === "left") list = list.filter((m) => !!m.left_at);
  if (watchFilter.value === "watched") list = list.filter((m) => isWatched(m.user_id));
  if (watchFilter.value === "unwatched") list = list.filter((m) => !isWatched(m.user_id));
  return list.sort((a, b) => {
    const aW = isWatched(a.user_id) ? 0 : 1;
    const bW = isWatched(b.user_id) ? 0 : 1;
    return aW - bW;
  });
});

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedMembers } = usePagination(tabFilteredMembers);

onMounted(() => { fetchMembers(); });
watch(selectedGuildId, () => { closeMember(); fetchMembers(); });

async function onSelectMember(userId: string) {
  await selectMember(userId);
}

function rolesCount(roles: unknown): number {
  return Array.isArray(roles) ? roles.length : 0;
}
</script>

<template>
  <AdminPageShell title="Membres" icon="👥" class="members-page">
    <template #actions>
      <span v-if="!loading" class="member-count">{{ tabFilteredMembers.length }} membres</span>
    </template>

    <div class="filters">
      <AppInput v-model="search" type="text" class="search-input" placeholder="Rechercher par nom ou ID..." />
      <AppSelect v-model="presenceFilter" class="sort-select">
        <option value="present">Présents sur le serveur</option>
        <option value="left">Partis</option>
        <option value="all">Tous (présents + partis)</option>
      </AppSelect>
      <AppSelect v-model="watchFilter" class="sort-select">
        <option value="all">Tous les membres</option>
        <option value="watched">Surveilles uniquement</option>
        <option value="unwatched">Non surveilles</option>
      </AppSelect>
      <AppSelect v-model="sortBy" class="sort-select">
        <option value="username">Tri par nom</option>
        <option value="joined_at">Tri par date d'arrivee</option>
      </AppSelect>
    </div>

    <div v-if="loading" class="loading">Chargement...</div>
    <ErrorState v-else-if="error" :message="error" :retryable="true" @retry="fetchMembers" />

    <div v-else class="content-layout">
      <!-- Left: list -->
      <div class="members-list">
        <div
          v-for="member in paginatedMembers"
          :key="member.user_id"
          :class="['card', 'member-card', { selected: selectedMember?.member.user_id === member.user_id }]"
          @click="onSelectMember(member.user_id)"
        >
          <div class="member-card-header">
            <div class="member-identity">
              <div class="avatar-placeholder member-avatar">{{ member.username.charAt(0).toUpperCase() }}</div>
              <div class="member-names">
                <span class="member-name">{{ member.display_name || member.username }}</span>
                <span class="member-id">{{ member.username }}</span>
              </div>
            </div>
            <div class="member-badges">
              <MemberStatusBadge :left-at="member.left_at" />
              <AppBadge v-if="isWatched(member.user_id)" label="SURVEILLE" variant="warning" />
            </div>
          </div>
          <div class="member-card-footer">
            <span>{{ rolesCount(member.roles) }} roles</span>
            <span>Depuis {{ formatDate(member.joined_at) }}</span>
          </div>
        </div>

        <div v-if="tabFilteredMembers.length === 0" class="empty">Aucun membre trouve</div>

        <PaginationBar
          :current-page="currentPage"
          :total-pages="totalPages"
          :total-items="totalItems"
          :per-page="perPage"
          @update:current-page="currentPage = $event"
          @update:per-page="perPage = $event"
        />
      </div>

      <!-- Right: detail drawer -->
      <MemberDetailDrawer v-if="selectedMember" />
      <div v-else class="card card--xl detail-placeholder">
        <div class="placeholder-icon">&#x1f465;</div>
        <p>Selectionnez un membre pour voir son profil</p>
      </div>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.member-count { font-size: 13px; color: var(--text-secondary); font-weight: 600; }

.filters {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}
.search-input::placeholder { color: var(--text-secondary); }
.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.sort-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 180px;
}
.sort-select:focus { outline: none; border-color: var(--accent); }

.loading, .empty { color: var(--text-secondary); padding: 40px; text-align: center; }

.content-layout { display: flex; gap: 20px; min-height: 0; }

.members-list {
  width: 720px;
  min-width: 720px;
  max-width: 720px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  max-height: calc(100vh - 240px);
  padding-right: 4px;
}

.member-card {
  padding: 14px 16px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.member-card:hover { border-color: var(--accent); background: var(--bg-hover); }
.member-card.selected { border-color: var(--accent); box-shadow: var(--focus-ring); }

.member-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.member-identity { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1; }
.member-avatar { width: 36px; height: 36px; font-size: 14px; flex-shrink: 0; }
.member-names { display: flex; flex-direction: column; gap: 1px; min-width: 0; flex: 1; }
.member-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.member-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.member-badges { display: flex; gap: 6px; }
.member-card-footer { display: flex; gap: 12px; font-size: 11px; color: var(--text-secondary); }

.detail-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
}
.placeholder-icon { font-size: 48px; margin-bottom: 12px; opacity: 0.5; }
.detail-placeholder p { font-size: 14px; }

@media (max-width: 900px) {
  .content-layout { flex-direction: column; }
  .members-list {
    width: 100%;
    min-width: 0;
    max-width: 100%;
    max-height: none;
  }
}

@media (max-width: 480px) {
  .sort-select { min-width: 0; width: 100%; }
  .search-input { width: 100%; }
}
</style>
