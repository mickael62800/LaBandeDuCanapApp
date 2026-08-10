<script setup lang="ts">
import { ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import MainLayout from "./components/templates/MainLayout.vue";
import PublicLayout from "./components/templates/PublicLayout.vue";
import ConfirmDialog from "./components/molecules/ConfirmDialog.vue";
import ToastContainer from "./components/molecules/ToastContainer.vue";

const route = useRoute();
const router = useRouter();

// Au tout premier render, route.name peut etre undefined (vue-router n'a
// pas encore fini sa premiere navigation -> route.meta.public est undefined,
// la branche v-else s'active par defaut, MainLayout monte, TopBar fetch
// /api/guilds AVANT l'auth -> 401 parasite. On bloque le render initial
// jusqu'a router.isReady() pour eviter ce flash de layout authentifie.
const ready = ref(false);
router.isReady().then(() => { ready.value = true; });
</script>

<template>
  <template v-if="ready">
    <!-- La mise en page suit `meta.layout`, plus `meta.public`. Les deux
         etaient confondus : toute page publique se retrouvait donc sans
         aucun chrome, ce qui laissait le site communautaire sans navigation
         et obligeait chaque page a redessiner sa propre barre. -->

    <!-- Connexion et retour OAuth : volontairement nus. -->
    <router-view v-if="route.meta.layout === 'bare'" />

    <!-- Site public communautaire. -->
    <PublicLayout v-else-if="route.meta.layout === 'site'">
      <router-view />
    </PublicLayout>

    <!-- Back-office (defaut). -->
    <MainLayout v-else>
      <router-view />
    </MainLayout>
  </template>

  <ConfirmDialog />
  <ToastContainer />
</template>
