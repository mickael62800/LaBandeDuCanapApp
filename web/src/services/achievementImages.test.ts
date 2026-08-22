import { describe, expect, it } from "vitest";
import { ACHIEVEMENT_IMAGES, imagesPourJeu, nomFichier } from "./achievementImages";

describe("ACHIEVEMENT_IMAGES", () => {
  it("expose les chemins publics des hauts faits Palworld", () => {
    expect(ACHIEVEMENT_IMAGES.palworld[0]).toBe("/Achievement/palworld/pal_01.jpg");
    // pal_31 et pal_42 manquent dans la liste : 54 fichiers au total.
    expect(ACHIEVEMENT_IMAGES.palworld).toHaveLength(54);
  });

  it("ne contient que des chemins locaux /Achievement/...", () => {
    for (const liste of Object.values(ACHIEVEMENT_IMAGES)) {
      for (const chemin of liste) {
        expect(chemin.startsWith("/Achievement/")).toBe(true);
      }
    }
  });
});

describe("imagesPourJeu", () => {
  it("renvoie les images du jeu connu", () => {
    const images = imagesPourJeu("palworld");
    expect(images).toHaveLength(54);
    expect(images[0]).toBe("/Achievement/palworld/pal_01.jpg");
  });

  it("renvoie un tableau vide pour un jeu inconnu", () => {
    expect(imagesPourJeu("eldenring")).toEqual([]);
  });

  it("renvoie un tableau vide quand le jeu est absent ou null", () => {
    expect(imagesPourJeu(null)).toEqual([]);
    expect(imagesPourJeu(undefined)).toEqual([]);
  });
});

describe("nomFichier", () => {
  it("extrait le nom de fichier d'un chemin public", () => {
    expect(nomFichier("/Achievement/palworld/pal_01.jpg")).toBe("pal_01.jpg");
  });

  it("renvoie la chaine telle quelle quand il n'y a pas de barre", () => {
    expect(nomFichier("pal_01.jpg")).toBe("pal_01.jpg");
  });
});
