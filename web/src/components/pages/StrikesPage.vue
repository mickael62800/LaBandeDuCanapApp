<script setup lang="ts">
import StrikesConfigForm from "../organisms/StrikesConfigForm.vue";
import StrikesUserLookup from "../organisms/StrikesUserLookup.vue";
</script>

<template>
  <!-- Contenu de SOUS-onglet (« Suivi utilisateur » > « Strikes ») :
       l'en-tete de page appartient a `ModerationHubPage`. Un `AdminPageShell`
       ici produisait un titre de page a deux niveaux sous les onglets. -->
  <div class="strikes-tab">
    <p class="tab-note">
      Escalade automatique : N strikes accumulés sur une fenêtre →
      action automatique (warn / mute / kick / ban). Les strikes expirent
      au bout de la fenêtre.
    </p>

    <StrikesConfigForm />
    <StrikesUserLookup />
  </div>
</template>

<style scoped>
.tab-note {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0 0 16px;
}
</style>
