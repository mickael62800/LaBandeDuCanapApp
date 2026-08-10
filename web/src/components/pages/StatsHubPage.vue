<script setup lang="ts">
// Hub Statistiques : serveur Discord + moderation, en onglets.
//
// C'est LE HUB qui porte l'en-tete de page. Les deux onglets embarquaient
// chacun leur propre `.dashboard-header` — titre degrade, bordure degradee,
// animation `stats-title-shimmer` — recopie a l'identique de l'un a l'autre.
// C'est d'ailleurs de cette page qu'`AdminPageShell` tient son style : le
// shell est desormais la reference, et le duplicata a disparu.

import { computed, ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import StatsPage from "./StatsPage.vue";
import ModstatsPage from "./ModstatsPage.vue";

type TabKey = "server" | "moderation";

const activeTab = ref<TabKey>("server");

const tabs = [
  {
    key: "server",
    label: "Serveur",
    lede: "Activité du serveur Discord : messages, membres, salons, vocal.",
  },
  {
    key: "moderation",
    label: "Modération",
    lede: "Activité de modération : sanctions, appels, charge par modérateur.",
  },
];

const activeLede = computed(
  () => tabs.find((t) => t.key === activeTab.value)?.lede ?? "",
);
</script>

<template>
  <AdminPageShell title="Statistiques" icon="📊" width="wide" class="stats-hub">
    <template #lede>{{ activeLede }}</template>

    <div class="hub-tabs-wrap">
      <AppTabs v-model="activeTab" :tabs="tabs" />
    </div>
    <StatsPage v-if="activeTab === 'server'" />
    <ModstatsPage v-else />
  </AdminPageShell>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
