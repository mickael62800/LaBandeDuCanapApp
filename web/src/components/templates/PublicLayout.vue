<script setup lang="ts">
// Mise en page du SITE PUBLIC communautaire (accueil, espace membre, jeux).
//
// Pendant de `MainLayout`, qui est celle du back-office. Comme elle, cette
// template ne fait que COMPOSER des organisms — elle ne contient pas de
// markup metier.
//
// Les deux mises en page ne partagent presque rien : ici pas de barre
// laterale, pas de WebSocket temps reel, pas de notifications, pas de
// selecteur de serveur. Un visiteur non connecte doit pouvoir consulter la
// page sans qu'aucun appel authentifie ne parte.
//
// `theme-communaute` est porte ici et non par chaque page : les trois pages
// publiques l'appliquaient chacune sur leur div racine.

import SiteHeader from "../organisms/SiteHeader.vue";
</script>

<template>
  <div class="site theme-communaute">
    <SiteHeader />
    <main class="site-main">
      <slot />
    </main>
  </div>
</template>

<style scoped>
.site {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.site-main {
  flex: 1;
  min-width: 0;
}
</style>
