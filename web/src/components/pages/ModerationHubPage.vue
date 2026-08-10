<script setup lang="ts">
// Hub Moderation : journal, bannis, suivi utilisateur, revue, rappels.
//
// C'est LE HUB qui porte l'en-tete de page. Ses onglets etaient de trois
// formes differentes : `AdminPageShell` complet (Revue, Rappels), simple
// `<h1>` (Preuves, Notes), ou rien du tout (Journal, Bannis). Changer
// d'onglet changeait donc la presence et la forme du titre.

import { computed, ref } from "vue";
import { useSharedUserLookup } from "../../composables/useSharedUserLookup";
import AppTabs from "../molecules/AppTabs.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import ModerationJournalTab from "../organisms/ModerationJournalTab.vue";
import ModerationBansTab from "../organisms/ModerationBansTab.vue";
import ModerationTrackingTab from "../organisms/ModerationTrackingTab.vue";
import ReviewPage from "./ReviewPage.vue";
import RemindersPage from "./RemindersPage.vue";

type TabKey = "journal" | "bans" | "tracking" | "review" | "reminders";

const activeTab = ref<TabKey>("journal");

const hubTabs = [
  {
    key: "journal",
    label: "Journal",
    lede: "Historique des sanctions appliquées sur le serveur.",
  },
  {
    key: "bans",
    label: "Bannis actifs",
    lede: "Membres actuellement bannis, avec le motif et l'auteur du ban.",
  },
  {
    key: "tracking",
    label: "Suivi utilisateur",
    lede: "Notes, preuves et avertissements rattachés à un membre.",
  },
  {
    key: "review",
    label: "Revue manuelle",
    lede: "Signalements en attente d'une décision humaine.",
  },
  {
    key: "reminders",
    label: "Rappels",
    lede: "Rappels programmés pour un suivi de modération.",
  },
];

const activeLede = computed(
  () => hubTabs.find((t) => t.key === activeTab.value)?.lede ?? "",
);

const { sharedUserId } = useSharedUserLookup();

// Incremente a chaque demande du Journal d'ouvrir Notes & Preuves : permet
// au TrackingTab de basculer son sous-onglet sans coupler les deux organismes.
const trackingJumpSignal = ref(0);

function handleOpenNotesEvidence(userId: string) {
  if (!userId) return;
  sharedUserId.value = userId;
  activeTab.value = "tracking";
  trackingJumpSignal.value++;
}
</script>

<template>
  <AdminPageShell title="Modération" icon="⚖️" class="moderation-hub">
    <template #lede>{{ activeLede }}</template>

    <AppTabs
      :model-value="activeTab"
      :tabs="hubTabs"
      class="hub-tabs-wrap"
      @update:model-value="(k) => (activeTab = k as TabKey)"
    />

    <div class="tab-content">
      <ModerationJournalTab
        v-if="activeTab === 'journal'"
        @open-notes-evidence="handleOpenNotesEvidence"
      />
      <ModerationBansTab v-else-if="activeTab === 'bans'" />
      <ModerationTrackingTab
        v-else-if="activeTab === 'tracking'"
        :jump-to-notes-evidence="trackingJumpSignal"
      />
      <ReviewPage v-else-if="activeTab === 'review'" />
      <RemindersPage v-else-if="activeTab === 'reminders'" />
    </div>
  </AdminPageShell>
</template>

<style scoped>
/* Le titre degrade vivait ici, en 3e exemplaire (apres `StatsPage` et
   `ModstatsPage`), avec sa propre animation `mod-title-shimmer`. Il vient
   desormais d'`AdminPageShell`. */
.hub-tabs-wrap { margin-bottom: 24px; }

.tab-content { animation: fadeSlideIn 0.3s ease-out; }
@keyframes fadeSlideIn {
  from { opacity: 0; transform: translateY(6px); }
  to   { opacity: 1; transform: translateY(0); }
}

@media (prefers-reduced-motion: reduce) {
  .tab-content { animation: none; }
}
</style>
