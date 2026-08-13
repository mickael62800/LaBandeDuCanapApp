import { createApp } from "vue";
import { MotionPlugin } from "@vueuse/motion";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import { routes } from "./router";
import { useAuth } from "./composables/useAuth";
import { initAppData, resetAppInit } from "./composables/useAppInit";
import { useGuildSelector } from "./composables/useGuildSelector";
import "./styles/global.css";

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// Bootstrap : la config API par defaut (origin courant + pas de Bearer)
// suffit en prod. L'OAuth Discord est gere par le backend, le front n'a
// rien a saisir. Cette fonction garantit qu'au moins une config existe.
import { setApiConfig, getApiConfig } from "./api/config";
import { loadSiteConfig } from "./siteConfig";
function ensureProdConfig() {
  if (!getApiConfig()) {
    setApiConfig({ api_url: window.location.origin });
  }
}

router.beforeEach(async (to, _from, next) => {
  ensureProdConfig();
  const { user, checkSession } = useAuth();
  await checkSession();
  if (!to.meta.public && !user.value) { next({ name: "login" }); return; }
  // Deja connecte et pourtant sur /login : la page n'a rien a offrir, on
  // redirige. La destination suit le parametre `espace` de l'URL, sinon un
  // administrateur cliquant « Se connecter » depuis l'espace membre se
  // retrouverait sur le tableau de bord sans l'avoir demande.
  //
  // Defaut : l'espace membre. Un membre ordinaire n'a pas acces au
  // back-office, et son role n'est pas encore resolu a cet instant.
  if (user.value && to.name === "login") {
    const vise = to.query.espace === "admin" ? "dashboard" : "membre";
    next({ name: vise });
    return;
  }

  // Prefetch async des donnees stables apres login. Non bloquant : on next()
  // immediatement. Les composables singleton (useBotDefinitions, useBotEnabledStatus)
  // auront leur cache rempli quand les pages les liront.
  if (user.value) {
    const { selectedGuildId } = useGuildSelector();
    const gid = selectedGuildId.value;
    if (gid) {
      void initAppData(gid);
      // Plus de guard de visibilite par route : l'acces au back-office est
      // reserve aux comptes de SUPERADMIN_USER_IDS, qui voient tout. Le
      // filtrage cote API reste la seule autorite (403).
    }
  } else {
    resetAppInit();
  }
  next();
});

const app = createApp(App);
app.use(createPinia());
app.use(router);
// Animations d'apparition au defilement (directive v-motion). Volontairement
// discretes : elles servent a guider la lecture de la page publique, pas a
// faire du spectacle. `prefers-reduced-motion` est respecte par la lib.
app.use(MotionPlugin);

// La configuration publique (guilde affichée, invitation Discord) est chargée
// AVANT le montage : la page membre la lit dès son `onMounted`, et l'attendre
// ici évite un premier rendu sans aucune section suivi d'un saut.
//
// Un échec n'empêche pas le montage : le site reste consultable, les sections
// publiques se masquent simplement.
loadSiteConfig().finally(() => app.mount("#app"));
