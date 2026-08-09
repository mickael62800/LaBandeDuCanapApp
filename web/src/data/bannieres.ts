// Catalogue des images livrées avec le site.
//
// Elles vivent dans `web/public/imgs/` et sont servies telles quelles. Le
// catalogue leur donne un libellé lisible et un regroupement : « Bienvenue »
// se choisit plus facilement que « bienvenue_banner.jpg », et personne n'a à
// connaître le nom des fichiers.
//
// Un test vérifie que ce catalogue et le dossier restent alignés — ni fichier
// oublié ici, ni entrée pointant vers une image absente.

export interface Banniere {
  /// Nom du fichier dans `public/imgs/`.
  fichier: string;
  libelle: string;
  /// Regroupement affiché dans le sélecteur.
  categorie: string;
}

export const BANNIERES: Banniere[] = [
  // ── Arrivées et départs ──
  { fichier: "bienvenue_banner.jpg", libelle: "Bienvenue", categorie: "Arrivées et départs" },
  { fichier: "bienvenue_general_banner.jpg", libelle: "Bienvenue (général)", categorie: "Arrivées et départs" },
  { fichier: "rebienvenue_banner.jpg", libelle: "Retour d'un ancien membre", categorie: "Arrivées et départs" },
  { fichier: "bye_banner.jpg", libelle: "Départ", categorie: "Arrivées et départs" },
  { fichier: "regle_banner.jpg", libelle: "Règlement", categorie: "Arrivées et départs" },
  { fichier: "reglement_embed_banner.jpg", libelle: "Règlement (illustré)", categorie: "Arrivées et départs" },

  // ── Communauté ──
  { fichier: "site_banner.jpg", libelle: "Bannière du site (La Bande du Canapé)", categorie: "Communauté" },
  { fichier: "anniv_serveur_banner.jpg", libelle: "Anniversaire du serveur", categorie: "Communauté" },
  { fichier: "anniv_membre_banner.jpg", libelle: "Anniversaire d'arrivée (année de plus)", categorie: "Communauté" },
  { fichier: "member_mois_banner.jpg", libelle: "Membre du mois", categorie: "Communauté" },
  { fichier: "member_annee_banner.jpg", libelle: "Membre de l'année", categorie: "Communauté" },
  { fichier: "nouveau_role_banner.jpg", libelle: "Nouveau rôle", categorie: "Communauté" },
  { fichier: "promo_staff.jpg", libelle: "Promotion au staff", categorie: "Communauté" },
  { fichier: "vip_banner.jpg", libelle: "VIP", categorie: "Communauté" },
  { fichier: "surprise_banner.jpg", libelle: "Surprise", categorie: "Communauté" },

  // ── Annonces et votes ──
  { fichier: "annonce_staff.jpg", libelle: "Annonce du staff", categorie: "Annonces et votes" },
  { fichier: "sondage_banner.jpg", libelle: "Sondage", categorie: "Annonces et votes" },
  { fichier: "vote_banner.jpg", libelle: "Vote", categorie: "Annonces et votes" },
  { fichier: "resultat_vote_banner.jpg", libelle: "Résultat de vote", categorie: "Annonces et votes" },

  // ── Événements ──
  { fichier: "gaming_night_banner.jpg", libelle: "Soirée jeux", categorie: "Événements" },
  { fichier: "vocal_night.jpg", libelle: "Soirée vocale", categorie: "Événements" },
  { fichier: "cherche_joueur_banner.jpg", libelle: "Recherche de joueurs", categorie: "Événements" },
  { fichier: "planning_semaine.jpg", libelle: "Planning de la semaine", categorie: "Événements" },
  { fichier: "hivers_banner.jpg", libelle: "Hiver", categorie: "Événements" },
  { fichier: "sun_banner.jpg", libelle: "Été", categorie: "Événements" },

  // ── Jeux ──
  { fichier: "minecraft_game.jpg", libelle: "Minecraft", categorie: "Jeux" },
  { fichier: "valheim_game.jpg", libelle: "Valheim", categorie: "Jeux" },
  { fichier: "palworld_game.jpg", libelle: "Palworld", categorie: "Jeux" },
  { fichier: "terraria_game.jpg", libelle: "Terraria", categorie: "Jeux" },
  { fichier: "factorio_game.jpg", libelle: "Factorio", categorie: "Jeux" },
  { fichier: "7days2die_game.jpg", libelle: "7 Days to Die", categorie: "Jeux" },
  { fichier: "core_keeper_game.jpg", libelle: "Core Keeper", categorie: "Jeux" },
  { fichier: "space_ingineers_game.jpg", libelle: "Space Engineers", categorie: "Jeux" },
  { fichier: "starbound_game.jpg", libelle: "Starbound", categorie: "Jeux" },
  { fichier: "vrising_game.jpg", libelle: "V Rising", categorie: "Jeux" },
  { fichier: "zomboid_game.jpg", libelle: "Project Zomboid", categorie: "Jeux" },

  // ── Utilitaires ──
  // Barre transparente tres large : posee comme image d'un embed, elle force
  // Discord a afficher la carte a sa largeur maximale (tous les embeds
  // deviennent alors identiques en largeur). Invisible a l'affichage.
  { fichier: "spacer_fullwidth.png", libelle: "Barre invisible (largeur fixe)", categorie: "Utilitaires" },
];

/// Chemin servi par le site, relatif à la racine.
export function cheminBanniere(fichier: string): string {
  return `/imgs/${fichier}`;
}

/**
 * URL ABSOLUE de la bannière.
 *
 * C'est celle qu'il faut enregistrer : ces images partent dans des embeds
 * Discord, et Discord va chercher l'image sur Internet. Un chemin relatif
 * (`/imgs/x.jpg`) ne lui dit rien — l'embed s'afficherait sans image, sans
 * erreur nulle part.
 */
export function urlBanniere(fichier: string): string {
  return `${window.location.origin}${cheminBanniere(fichier)}`;
}

/**
 * Retrouve la bannière correspondant à une URL enregistrée.
 *
 * Compare sur le seul nom de fichier : l'URL stockée porte le domaine du jour
 * où elle a été choisie. Changer de nom de domaine ne doit pas faire
 * « oublier » au sélecteur quelle image est sélectionnée.
 */
export function banniereDepuisUrl(url: string | null | undefined): Banniere | null {
  if (!url) return null;
  const fichier = url.split("/").pop() ?? "";
  return BANNIERES.find((b) => b.fichier === fichier) ?? null;
}

/// Catégories dans leur ordre de déclaration, sans doublon.
export function categories(): string[] {
  return [...new Set(BANNIERES.map((b) => b.categorie))];
}
