// Registre des univers applicatifs — source unique de verite.
//
// Trois produits distincts partagent ce back-office : Sentinel (moderation et
// communaute), Nexus (plateforme jeux) et Atrium (accueil assiste par IA).
// Ils ont chacun leur backend, mais la meme identite Discord et le meme RBAC.
//
// POURQUOI UN REGISTRE ET PAS UNE SUITE DE CONDITIONS
//
// L'univers etait auparavant DEDUIT de l'URL — `path.startsWith("/nexus")
// ? "nexus" : "sentinel"` — ce qui faisait de Sentinel non pas un univers
// mais le `else` de Nexus. Trois consequences vecues :
//   - impossible d'ajouter un 3e univers sans reecrire chaque condition ;
//   - les pages publiques (/membre, /jeux), qui ne commencent pas par
//     /nexus, basculaient silencieusement l'application en Sentinel ;
//   - chaque composant reimplementait « quelle est la page d'accueil de cet
//     univers ? », d'ou un logo qui ramenait toujours sur /dashboard et
//     faisait donc SORTIR de Nexus.
//
// Desormais une route DECLARE son univers (`meta.universe`) et tout le reste
// — marque, accent, page d'accueil — se lit ici. Ajouter un univers = une
// entree dans ce fichier + `meta.universe` sur ses routes.
//
// L'univers n'est PAS un droit : il ne fait que structurer la navigation.
// Les acces restent gardes cote serveur (gate RBAC `nexus.access` verifie par
// la passerelle nginx avant chaque appel a nexus-api).

import { ATRIUM, NEXUS, OPS, SENTINEL, type Brand } from "./branding";

// LE 4e UNIVERS N'EST PAS UN PRODUIT
//
// Sentinel, Nexus et Atrium sont trois produits Discord. « Exploitation » est
// d'une autre nature : c'est la MACHINE qui les heberge — conteneurs Docker,
// disques, certificats TLS, IP bannies, logs des services. Ces ecrans etaient
// ranges dans « Configuration » chez Sentinel, alors qu'ils ne parlent pas de
// Discord et concernent autant Nexus et Atrium.
//
// Attention au mot « serveur », qui designe ici TROIS choses differentes : le
// serveur Discord (guilde), le serveur de jeu (Nexus) et la machine hote.
// Les libelles de navigation doivent lever l'ambiguite, jamais l'entretenir.

export type UniverseKey = "sentinel" | "nexus" | "atrium" | "ops";

export interface UniverseDef {
  key: UniverseKey;
  /// Identite visuelle affichee dans la barre du haut.
  brand: Brand;
  /// Couleur d'accent de l'univers, injectee en `--universe-accent`.
  accent: string;
  /// Page d'accueil : ou l'on atterrit en basculant vers cet univers, et ou
  /// mene le logo. Doit exister dans le routeur.
  home: string;
}

/// Accents volontairement ECARTES les uns des autres.
///
/// Sentinel (#5865f2) et Nexus (#7c5cfc) etaient auparavant deux bleu-violets
/// a un cran d'ecart : a l'usage, rien ne signalait le changement de produit.
/// Une couleur d'univers n'a d'utilite que si elle est reconnaissable seule,
/// d'ou trois teintes franchement distinctes.
export const UNIVERSES: Record<UniverseKey, UniverseDef> = {
  sentinel: {
    key: "sentinel",
    brand: SENTINEL,
    accent: "#5865f2",
    home: "/dashboard",
  },
  nexus: {
    key: "nexus",
    brand: NEXUS,
    accent: "#a855f7",
    home: "/nexus/servers",
  },
  atrium: {
    key: "atrium",
    brand: ATRIUM,
    accent: "#14b8a6",
    home: "/atrium",
  },
  ops: {
    key: "ops",
    brand: OPS,
    accent: "#f59e0b",
    home: "/server-health",
  },
};

/// Ordre d'affichage de la bascule d'univers. Exploitation en dernier : on y
/// va pour diagnostiquer, pas pour travailler au quotidien.
export const UNIVERSE_ORDER: UniverseKey[] = [
  "sentinel",
  "nexus",
  "atrium",
  "ops",
];

/// Univers par defaut quand la route n'en declare aucun (page publique,
/// premiere navigation). Ne sert qu'a l'affichage : aucune route
/// d'administration ne doit compter dessus — elles declarent toutes le leur.
export const DEFAULT_UNIVERSE: UniverseKey = "sentinel";

export function isUniverseKey(value: unknown): value is UniverseKey {
  return value === "sentinel" || value === "nexus" || value === "atrium";
}

export function universeDef(key: UniverseKey): UniverseDef {
  return UNIVERSES[key];
}
