// Composition des deux racines de l'application.
//
// SITE PUBLIC et BACK-OFFICE sont deux produits differents, avec deux publics,
// deux mises en page et deux modeles d'authentification. Ils partagent un seul
// build et un seul deploiement, mais plus une seule liste de routes a plat ou
// l'accueil communautaire cotoyait la configuration des conteneurs Docker.
//
// `meta.public` ne signifie plus que « accessible sans connexion ». Il servait
// auparavant AUSSI a choisir la mise en page, ce qui rendait impossible une
// page publique avec chrome ou une page authentifiee sans chrome. C'est
// desormais le role de `meta.layout`.

import type { RouteRecordRaw } from "vue-router";
import type { UniverseKey } from "@/universes";
import { publicRoutes } from "./publicRoutes";
import { adminRoutes } from "./adminRoutes";

/// Mise en page a appliquer. Absente = back-office (le cas le plus frequent).
///   - `site` : site public communautaire (en-tete + navigation publique)
///   - `bare` : aucun chrome (connexion, retour OAuth)
export type RouteLayout = "site" | "bare";

declare module "vue-router" {
  interface RouteMeta {
    /// Accessible sans connexion. Propriete d'AUTHENTIFICATION uniquement :
    /// elle ne dit rien de la mise en page.
    public?: boolean;
    /// Mise en page. Absente = back-office.
    layout?: RouteLayout;
    /// Univers d'administration de la route. Absent = hors back-office : la
    /// navigation conserve alors le dernier univers visite. Toute route
    /// d'administration DOIT le declarer (cf. `universes.ts`).
    universe?: UniverseKey;
  }
}

/// Anciennes URLs devenues des onglets de hub. Conservees pour ne pas casser
/// les favoris : les pages cibles existent toujours, elles sont simplement
/// affichees comme onglets de leur hub.
const legacyRedirects: RouteRecordRaw[] = [
  { path: "/modstats", redirect: "/stats" },
  { path: "/strikes", redirect: "/moderation" },
  { path: "/notes", redirect: "/moderation" },
  { path: "/reminders", redirect: "/moderation" },
  { path: "/evidence", redirect: "/moderation" },
  { path: "/review", redirect: "/moderation" },
  { path: "/voice-themes", redirect: "/voice-channels" },
  { path: "/levels-config", redirect: "/levels" },
  { path: "/watched-users", redirect: "/members" },
];

export const routes: RouteRecordRaw[] = [
  ...publicRoutes,
  ...adminRoutes,
  ...legacyRedirects,
];
