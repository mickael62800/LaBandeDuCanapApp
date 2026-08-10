// Racine PUBLIQUE : le site communautaire et le parcours de connexion.
//
// Rien ici n'appartient a un univers d'administration — ces pages n'en
// declarent donc aucun, et la navigation conserve le dernier univers visite
// (cf. `useUniverse`).
//
// Deux mises en page differentes cohabitent dans ce fichier :
//   - `site` : le site communautaire, avec son en-tete et sa navigation.
//   - `bare` : connexion et retour OAuth, volontairement sans aucun chrome —
//     une barre de navigation sur un ecran de connexion n'offre que des
//     destinations qui redemanderaient de se connecter.

import type { RouteRecordRaw } from "vue-router";

// Eager : critiques au boot, ces deux pages sont le point d'entree de toute
// session. Les lazy-loader ajouterait un aller-retour reseau avant meme de
// pouvoir se connecter.
import LoginPage from "@/components/pages/auth/LoginPage.vue";
import AuthCallbackPage from "@/components/pages/auth/AuthCallbackPage.vue";

export const publicRoutes: RouteRecordRaw[] = [
  {
    path: "/login",
    name: "login",
    component: LoginPage,
    meta: { public: true, layout: "bare" },
  },
  {
    path: "/auth/callback",
    name: "auth-callback",
    component: AuthCallbackPage,
    meta: { public: true, layout: "bare" },
  },

  // Accueil PUBLIC du site communautaire : visible sans connexion. Le
  // back-office demarre a /dashboard.
  {
    path: "/",
    name: "public-home",
    component: () => import("@/components/pages/public/PublicHomePage.vue"),
    meta: { public: true, layout: "site" },
  },

  // Espace MEMBRE : PUBLIC. Un visiteur doit pouvoir voir ce qui se passe
  // (planning, evenements en cours) avant de decider de creer un compte —
  // demander la connexion a l'entree revenait a mettre un videur devant une
  // vitrine. La connexion n'est requise que pour AGIR (s'inscrire).
  {
    path: "/membre",
    name: "membre",
    component: () => import("@/components/pages/public/MemberHomePage.vue"),
    meta: { public: true, layout: "site" },
  },

  // Publique comme l'espace membre : un visiteur voit la roue et le
  // classement, la connexion n'est exigee que pour jouer.
  {
    path: "/jeux",
    name: "jeux",
    component: () => import("@/components/pages/public/GamesPage.vue"),
    meta: { public: true, layout: "site" },
  },
];
