// Images de hauts faits livrees avec le dashboard.
//
// Les fichiers vivent dans `web/public/Achievement/<jeu>/` et sont servis tels
// quels a la racine du site. On les enregistre en base sous forme de CHEMIN
// LOCAL (`/Achievement/palworld/pal_01.jpg`) et non d'URL d'asset construite
// par Vite : un chemin public reste stable d'un build a l'autre, alors qu'une
// URL hachee changerait a chaque deploiement et casserait les images deja
// choisies.
//
// Cette liste est explicite pour la meme raison : `import.meta.glob` ne voit
// pas `public/`, et globber le dossier produirait des URL hachees. Ajouter une
// image = deposer le fichier puis ajouter son nom ici.

const PALWORLD_FILES = [
  "pal_01.jpg",
  "pal_02.jpg",
  "pal_03.jpg",
  "pal_04.jpg",
  "pal_05.jpg",
  "pal_06.jpg",
  "pal_07.jpg",
  "pal_08.jpg",
  "pal_09.jpg",
  "pal_10.jpg",
  "pal_11.jpg",
  "pal_12.jpg",
  "pal_13.jpg",
  "pal_14.jpg",
  "pal_15.jpg",
  "pal_16.jpg",
  "pal_17.jpg",
  "pal_18.jpg",
  "pal_19.jpg",
  "pal_20.jpg",
  "pal_21.jpg",
  "pal_22.jpg",
  "pal_23.jpg",
  "pal_24.jpg",
  "pal_25.jpg",
  "pal_26.jpg",
  "pal_27.jpg",
  "pal_28.jpg",
  "pal_29.jpg",
  "pal_30.jpg",
  "pal_32.jpg",
  "pal_33.jpg",
  "pal_34.jpg",
  "pal_35.jpg",
  "pal_36.jpg",
  "pal_37.jpg",
  "pal_38.jpg",
  "pal_39.jpg",
  "pal_40.jpg",
  "pal_42.jpg",
  "pal_43.jpg",
  "pal_44.jpg",
  "pal_45.jpg",
  "pal_46.jpg",
  "pal_47.jpg",
  "pal_48.jpg",
  "pal_49.jpg",
  "pal_50.jpg",
  "pal_51.jpg",
  "pal_52.jpg",
  "pal_53.jpg",
  "pal_54.jpg",
  "pal_55.jpg",
  "pal_56.jpg",
] as const;

const base = (jeu: string, fichier: string) => `/Achievement/${jeu}/${fichier}`;

/** Images disponibles par slug de jeu. */
export const ACHIEVEMENT_IMAGES: Record<string, string[]> = {
  palworld: PALWORLD_FILES.map((f) => base("palworld", f)),
};

/** Images proposees pour un jeu ; tableau vide si le jeu n'en fournit pas. */
export function imagesPourJeu(game: string | null | undefined): string[] {
  if (!game) return [];
  return ACHIEVEMENT_IMAGES[game] ?? [];
}

/** Nom de fichier seul, pour un affichage compact dans un select. */
export function nomFichier(chemin: string): string {
  return chemin.split("/").pop() ?? chemin;
}
