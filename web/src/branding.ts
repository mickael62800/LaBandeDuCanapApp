// Identites visuelles du site — source unique.
//
// Trois marques distinctes cohabitent :
//   - COMMUNITY : la communaute elle-meme (La Bande du Canape). C'est la
//     marque du site public et de la page de connexion : un visiteur arrive
//     chez la communaute, pas chez un outil d'administration.
//   - SENTINEL  : le back-office moderation / communaute.
//   - NEXUS     : la plateforme jeux.
//
// Chaque marque a DEUX declinaisons, comme toute identite visuelle serieuse :
//   - `mark`     : le symbole seul, sans texte. Pour les petites tailles
//                  (barre du haut, favicon) ou le texte serait illisible.
//   - `wordmark` : la version avec le nom ecrit. Pour les grands formats
//                  (accueil public, page de connexion) ou elle porte l'identite.
//
// Regrouper les chemins ici evite les references en dur dispersees dans les
// composants — c'est precisement ce qui avait laisse un `/logo.png`
// inexistant reference a trois endroits + le favicon.

import { siteConfig } from "./siteConfig";

export interface Brand {
  /// Nom affiche.
  name: string;
  /// Symbole seul, sans texte — petites tailles.
  mark: string;
  /// Version avec le nom ecrit — grands formats. Absente = on retombe sur
  /// `mark`, ce qui reste correct visuellement.
  wordmark?: string;
  /// Phrase d'accroche, utilisee sous le titre.
  tagline: string;
}

/// Invitation Discord de la communaute.
///
/// Lue a l'execution depuis `site-config.json` (cf. `siteConfig.ts`), avec
/// repli sur `VITE_DISCORD_INVITE` fixe au build pour le developpement local.
/// Corriger un lien ne doit pas obliger a reconstruire l'image.
///
/// Absente, le bouton « Rejoindre Discord » n'est pas affiche : un bouton qui
/// ne mene nulle part est pire que pas de bouton.
export function discordInvite(): string {
  return (
    siteConfig().discordInvite ||
    ((import.meta.env.VITE_DISCORD_INVITE as string | undefined) ?? "")
  );
}

export const COMMUNITY: Brand = {
  name: "La Bande du Canapé",
  mark: "/canape_mark.png",
  wordmark: "/canape_wordmark.png",
  tagline: "Le serveur où l'on se pose, on joue, et on reste.",
};

export const SENTINEL: Brand = {
  name: "Sentinel",
  mark: "/sentinel_logo.png",
  tagline: "Moderation et communaute",
};

export const NEXUS: Brand = {
  name: "Nexus",
  mark: "/nexus_logo.png",
  tagline: "Plateforme jeux",
};

export const ATRIUM: Brand = {
  name: "Atrium",
  mark: "/atrium_logo.png",
  tagline: "Accueil assiste par IA",
};

/// Exploitation : la machine hote, pas un produit Discord. Pas de logo
/// dedie — `onLogoError` masque proprement l'image absente, il n'y a donc
/// rien a fournir tant qu'on ne veut pas d'icone ici.
export const OPS: Brand = {
  name: "Exploitation",
  mark: "/ops_logo.png",
  tagline: "Machine, services et securite de l'hote",
};

/// Logo grand format : le wordmark s'il existe, sinon le symbole seul.
export function wordmarkOf(brand: Brand): string {
  return brand.wordmark ?? brand.mark;
}

/// Repli en cascade pour un logo grand format : si le wordmark n'est pas
/// encore fourni, on affiche le symbole seul plutot que rien. L'accueil reste
/// donc presentable avant meme la livraison du logo avec texte.
export function onWordmarkError(event: Event, brand: Brand): void {
  const el = event.target as HTMLImageElement | null;
  if (!el) return;
  if (brand.wordmark && el.dataset.fallback !== "1") {
    el.dataset.fallback = "1";
    el.src = brand.mark;
    return;
  }
  el.style.display = "none";
}

/// Masque l'image si le fichier n'existe pas encore (logo pas encore fourni).
/// Sans ca, le navigateur affiche une icone de lien casse.
export function onLogoError(event: Event): void {
  const el = event.target as HTMLImageElement | null;
  if (el) el.style.display = "none";
}
