<script setup lang="ts">
// Hub Niveaux : classement + configuration, en onglets.
//
// C'est LE HUB qui porte l'en-tete de page. Les deux onglets ne s'accordaient
// meme pas entre eux : « Niveaux & XP » etait un simple `<h1>`, «Niveaux —
// configuration » un `AdminPageShell` complet. Changer d'onglet changeait donc
// la forme ET la taille du titre.

import { computed, ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import LevelsPage from "./LevelsPage.vue";
import LevelsConfigPage from "./LevelsConfigPage.vue";

type TabKey = "leaderboard" | "config";

const activeTab = ref<TabKey>("leaderboard");

const tabs = [
  {
    key: "leaderboard",
    label: "Classement",
    lede: "Classement des membres par XP, alimenté en temps réel.",
  },
  {
    key: "config",
    label: "Configuration",
    lede: "Paliers de rôles et ajustement manuel de l'XP.",
  },
];

const activeLede = computed(
  () => tabs.find((t) => t.key === activeTab.value)?.lede ?? "",
);
</script>

<template>
  <AdminPageShell title="Niveaux &amp; XP" icon="📈" class="levels-hub">
    <template #lede>{{ activeLede }}</template>

    <div class="hub-tabs-wrap">
      <AppTabs v-model="activeTab" :tabs="tabs" />
    </div>
    <LevelsPage v-if="activeTab === 'leaderboard'" />
    <LevelsConfigPage v-else />
  </AdminPageShell>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
