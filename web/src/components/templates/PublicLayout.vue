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
/* `#app` est un conteneur FLEX EN LIGNE de 100vh (global.css) : ce reglage a
   ete pense pour le back-office, ou `MainLayout` place la barre laterale et le
   contenu cote a cote. Le site public est donc, lui aussi, un ITEM de ce flex
   en ligne — et un item vaut `flex: 0 1 auto` par defaut, c'est-a-dire la
   largeur de son contenu, pas celle de l'ecran.
   Resultat : le site se tassait a gauche sur toute sa hauteur, les conteneurs
   internes (`max-width: 68rem`) n'ayant rien a remplir, et ce qui depassait
   etait coupe a droite sans barre de defilement — `body` est en
   `overflow: hidden`. `flex: 1` reclame la largeur ; `min-width: 0` autorise
   la reduction sous la taille du contenu, sans quoi un enfant large (tableau,
   embed) repousserait a nouveau le bord droit hors de l'ecran.
   Meme trio que `.main-wrapper` / `.main-content` cote back-office. */
.site {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

/* Le defilement appartient a cette zone et non au document : `body` etant en
   `overflow: hidden`, une page plus haute que 100vh serait sinon tronquee en
   bas, sans moyen d'atteindre le reste. La barre du site reste visible sans
   rien devoir a son `position: sticky` : elle est SOEUR de cette zone, donc
   hors du defilement. */
.site-main {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}
</style>
