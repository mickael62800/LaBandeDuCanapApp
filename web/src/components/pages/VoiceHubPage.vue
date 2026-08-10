<script setup lang="ts">
// Hub Vocaux : salons temporaires + themes, en onglets.
//
// C'est LE HUB qui porte l'en-tete de page (titre + descriptif), pas ses
// onglets. Chaque onglet embarquait auparavant son propre `AdminPageShell` :
// on obtenait un grand titre DIFFERENT sous la barre d'onglets a chaque
// changement d'onglet, comme si l'on changeait de page alors qu'on reste sur
// la meme. Le descriptif, lui, est bien propre a l'onglet : il suit.

import { computed, ref } from "vue";
import AppTabs from "../molecules/AppTabs.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import VoiceChannelsPage from "./VoiceChannelsPage.vue";
import VoiceThemesPage from "./VoiceThemesPage.vue";

type TabKey = "channels" | "themes";

const activeTab = ref<TabKey>("channels");

const tabs = [
  {
    key: "channels",
    label: "Salons",
    lede: "Salons vocaux temporaires actifs.",
  },
  {
    key: "themes",
    label: "Thèmes",
    lede:
      "Gabarits de salons vocaux temporaires (nom, limite, bitrate, visibilité, " +
      "slowmode, queue, stage). Quand un membre rejoint le salon trigger configuré, " +
      "le bot crée un salon dérivé du thème par défaut.",
  },
];

const activeLede = computed(
  () => tabs.find((t) => t.key === activeTab.value)?.lede ?? "",
);
</script>

<template>
  <AdminPageShell title="Vocaux" icon="🎙️" class="voice-hub">
    <template #lede>{{ activeLede }}</template>

    <div class="hub-tabs-wrap">
      <AppTabs v-model="activeTab" :tabs="tabs" />
    </div>
    <VoiceChannelsPage v-if="activeTab === 'channels'" />
    <VoiceThemesPage v-else />
  </AdminPageShell>
</template>

<style scoped>
.hub-tabs-wrap {
  margin-bottom: 20px;
}
</style>
