<script setup lang="ts">
// Hub Rôles : panneaux d'auto-attribution + rôles Discord, en onglets.
//
// C'est LE HUB qui porte l'en-tete de page. Les deux onglets affichaient
// chacun leur titre ET un lien croise vers l'autre (« Voir tous les roles
// Discord → », « ← Panels de roles ») : une navigation en doublon de la barre
// d'onglets, heritee de l'epoque ou c'etaient deux pages separees.

import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import AppTabs from "../molecules/AppTabs.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import RolePanelsPage from "./RolePanelsPage.vue";
import DiscordRolesPage from "./DiscordRolesPage.vue";

type TabKey = "panels" | "roles";
const tabs = [
  {
    key: "panels",
    label: "Panneaux de rôles",
    lede: "Panneaux d'auto-attribution de rôles postés sur Discord.",
  },
  {
    key: "roles",
    label: "Rôles Discord",
    lede: "Tous les rôles du serveur, avec leur hiérarchie et leurs membres.",
  },
];

const route = useRoute();
const router = useRouter();

// L'onglet actif est dérivé de l'URL : /discord-roles ouvre "roles", sinon
// "panels". Ainsi le lien croisé "Voir tous les rôles" continue de fonctionner,
// et l'onglet reste bookmarkable.
function tabFromPath(path: string): TabKey {
  return path.startsWith("/discord-roles") ? "roles" : "panels";
}
const activeTab = ref<TabKey>(tabFromPath(route.path));

watch(
  () => route.path,
  (p) => {
    activeTab.value = tabFromPath(p);
  },
);

// Cliquer un onglet met à jour l'URL (sans empiler l'historique).
function onTabChange(key: string) {
  const target = key === "roles" ? "/discord-roles" : "/role-panels";
  if (route.path !== target) router.replace(target);
  activeTab.value = key as TabKey;
}

const activeLede = computed(
  () => tabs.find((t) => t.key === activeTab.value)?.lede ?? "",
);
</script>

<template>
  <AdminPageShell title="Rôles" icon="🎭" class="roles-hub">
    <template #lede>{{ activeLede }}</template>

    <div class="hub-tabs-wrap">
      <AppTabs :model-value="activeTab" :tabs="tabs" @update:model-value="onTabChange" />
    </div>
    <RolePanelsPage v-if="activeTab === 'panels'" />
    <DiscordRolesPage v-else />
  </AdminPageShell>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
