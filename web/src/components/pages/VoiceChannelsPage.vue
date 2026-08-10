<script setup lang="ts">
import { ref } from "vue";
import { useVoiceChannels, useVoiceChannelDetail } from "../../composables/useVoiceChannels";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import VoiceChannelDetailPanel from "../organisms/VoiceChannelDetailPanel.vue";
import VoiceChannelsActiveList from "../organisms/VoiceChannelsActiveList.vue";
import VoiceChannelsHistoryList from "../organisms/VoiceChannelsHistoryList.vue";

// Note : useVoiceChannels n'est PAS module-scoped — chaque appel crée son
// propre state. La page et les organisms enfants appellent tous
// useVoiceChannels(), donc en pratique chacun a son propre cache. Ici on
// l'invoque uniquement pour le KPI bar et les realtime refreshes ; le
// vrai fetch / data se fait dans chaque organism.
const {
  publicCount,
  privateCount,
  totalCount,
  fetchChannels,
  fetchHistory,
} = useVoiceChannels();

const { detail, events, loading: detailLoading, eventsLoading, fetchDetail } =
  useVoiceChannelDetail();

useRealtimeRefresh(
  [
    "voice_channel_created",
    "voice_channel_closed",
    "voice_channel_updated",
    "voice_invite_created",
    "voice_invite_used",
    "voice_invite_revoked",
  ],
  async () => {
    await Promise.all([fetchChannels(), fetchHistory()]);
  },
);

const selectedId = ref<string | null>(null);

async function selectChannel(channelId: string) {
  selectedId.value = channelId;
  await fetchDetail(channelId);
}

function backToList() {
  selectedId.value = null;
  detail.value = null;
}
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `VoiceHubPage`. -->
  <div class="voice-channels-tab">
    <div class="stats-row">
      <div class="stat-card">
        <span class="stat-value">{{ totalCount }}</span>
        <span class="stat-label">Total</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{{ publicCount }}</span>
        <span class="stat-label">Public</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{{ privateCount }}</span>
        <span class="stat-label">Prive</span>
      </div>
    </div>

    <VoiceChannelDetailPanel
      v-if="selectedId"
      :detail="detail"
      :events="events"
      :detail-loading="detailLoading"
      :events-loading="eventsLoading"
      @back="backToList"
    />

    <template v-else>
      <VoiceChannelsActiveList @select="selectChannel" />
      <VoiceChannelsHistoryList @select="selectChannel" />
    </template>
  </div>
</template>

<style scoped>
.stats-row {
  display: flex;
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 120px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

@media (max-width: 768px) {
  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
  }
  .stat-card {
    min-width: 0;
    padding: 10px 14px;
  }
  .stat-value { font-size: 22px; }
}
</style>
