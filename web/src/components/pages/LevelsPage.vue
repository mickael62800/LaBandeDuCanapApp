<script setup lang="ts">
import { useLevels } from "../../composables/useLevels";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import ErrorState from "../atoms/ErrorState.vue";
import LevelsLeaderboardTab from "../organisms/LevelsLeaderboardTab.vue";

const { loading, error, fetchAll } = useLevels();
useRealtimeRefresh(["xp_gained", "xp_admin_set", "xp_admin_reset"], fetchAll);
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `LevelsHubPage`. -->
  <div class="levels">
    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchAll" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else>
      <LevelsLeaderboardTab />
    </template>
  </div>
</template>

<style scoped>
.loading {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
