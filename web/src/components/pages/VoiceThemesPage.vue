<script setup lang="ts">
import { ref } from "vue";
import VoiceThemesTable from "../organisms/VoiceThemesTable.vue";
import VoiceThemeFormModal from "../organisms/VoiceThemeFormModal.vue";
import type { VoiceChannelTheme } from "@/types/voice-extended";

const showForm = ref(false);
const editing = ref<VoiceChannelTheme | null>(null);

function onCreate() {
  editing.value = null;
  showForm.value = true;
}
function onEdit(t: VoiceChannelTheme) {
  editing.value = t;
  showForm.value = true;
}
function onClose() {
  showForm.value = false;
  editing.value = null;
}
</script>

<template>
  <!-- Contenu d'onglet : l'en-tete de page appartient a `VoiceHubPage`. -->
  <div class="voice-themes-tab">
    <p class="tab-note">
      Variables disponibles : <code>{username}</code>, <code>{theme}</code>.
    </p>

    <VoiceThemesTable @create="onCreate" @edit="onEdit" />
    <VoiceThemeFormModal :open="showForm" :editing="editing" @close="onClose" />
  </div>
</template>

<style scoped>
@import "./_admin-page-shared.css";

.tab-note {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0 0 16px;
}
</style>
