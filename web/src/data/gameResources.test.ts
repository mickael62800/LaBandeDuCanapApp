import { describe, expect, it } from "vitest";
import { gameResources, trouverParSlug } from "./gameResources";

/// Slugs reellement presents dans `game_templates` (migrations nexus 007, 054,
/// 055, 056). Une fiche dont le slug ne correspond a aucun template ne
/// s'afficherait JAMAIS sur la page de creation : le guide retomberait
/// silencieusement sur le catalogue complet, et personne ne verrait l'erreur.
const SLUGS_DU_CATALOGUE = [
  "minecraft-vanilla",
  "valheim",
  "factorio",
  "palworld",
  "ark",
  "7dtd",
  "terraria",
  "enshrouded",
  "satisfactory",
  "project-zomboid",
  "vrising",
  "core-keeper",
  "necesse",
  "vintage-story",
];

/// Plafond que docker-agent fait respecter quoi que demande l'API
/// (`DOCKER_AGENT_MAX_CPU_LIMIT`, defaut 16 — voir bollard_game.rs). Conseiller
/// au-dela ferait echouer la creation du conteneur, avec une erreur que rien
/// dans le formulaire ne laissait prevoir.
const MAX_VCPU_AGENT = 16;
const MIN_VCPU_AGENT = 0.5;

/// Bornes memoire de l'agent : 512 Mo a 24 Go (bollard_game.rs).
const MIN_RAM_GO = 0.5;
const MAX_RAM_GO = 24;

/// « 6-8 » ou « 24+ » : on en extrait les bornes numeriques.
function bornesRam(ram: string): number[] {
  return ram
    .replace(",", ".")
    .split(/[-+]/)
    .map((p) => p.trim())
    .filter(Boolean)
    .map(Number);
}

describe("integrite du catalogue", () => {
  it("couvre exactement les jeux du catalogue, sans oubli ni fiche orpheline", () => {
    // Un jeu ajoute en base sans fiche ici n'a aucune recommandation, et une
    // fiche sans template en base ne s'affiche jamais.
    expect([...gameResources.map((g) => g.slug)].sort()).toEqual(
      [...SLUGS_DU_CATALOGUE].sort(),
    );
  });

  it("n'a pas deux fiches pour le meme jeu", () => {
    // `trouverParSlug` renvoie la premiere : un doublon rendrait la seconde
    // invisible sans que rien ne le signale.
    const slugs = gameResources.map((g) => g.slug);
    expect(new Set(slugs).size).toBe(slugs.length);
  });

  it("donne a chaque jeu un nom, une icone et au moins un facteur", () => {
    for (const jeu of gameResources) {
      expect(jeu.name.trim(), jeu.slug).not.toBe("");
      expect(jeu.icon.trim(), jeu.slug).not.toBe("");
      // Les facteurs sont la vraie information de la fiche : le tableau seul
      // ferait recopier une ligne sans comprendre ce qui la fait varier.
      expect(jeu.facteurs.length, jeu.slug).toBeGreaterThan(0);
    }
  });
});

describe("coherence des recommandations", () => {
  it("propose au moins deux paliers par jeu", () => {
    // Un palier unique ne dit pas comment la consommation evolue : c'est
    // precisement ce que le tableau est cense montrer.
    for (const jeu of gameResources) {
      expect(jeu.recommendations.length, jeu.slug).toBeGreaterThanOrEqual(2);
    }
  });

  it("classe les paliers par nombre de joueurs croissant", () => {
    // La page de documentation lit la PREMIERE et la DERNIERE ligne pour
    // annoncer une fourchette : desordonnees, elle annoncerait l'inverse.
    for (const jeu of gameResources) {
      const joueurs = jeu.recommendations.map((r) => r.players);
      expect(joueurs, jeu.slug).toEqual([...joueurs].sort((a, b) => a - b));
    }
  });

  it("ne repete pas un meme nombre de joueurs dans un jeu", () => {
    // Le tableau utilise `players` comme cle de rendu : un doublon ferait
    // disparaitre une ligne.
    for (const jeu of gameResources) {
      const joueurs = jeu.recommendations.map((r) => r.players);
      expect(new Set(joueurs).size, jeu.slug).toBe(joueurs.length);
    }
  });

  it("ne demande jamais moins de memoire pour plus de joueurs", () => {
    for (const jeu of gameResources) {
      const minima = jeu.recommendations.map((r) => Math.min(...bornesRam(r.ram_gb)));
      for (let i = 1; i < minima.length; i++) {
        expect(
          minima[i]!,
          `${jeu.slug} : ${jeu.recommendations[i]!.players} joueurs`,
        ).toBeGreaterThanOrEqual(minima[i - 1]!);
      }
    }
  });

  it("tient la memoire dans les bornes que docker-agent accepte", () => {
    // Au-dela, la creation du conteneur echoue : conseiller une valeur
    // irrecevable est pire que ne rien conseiller.
    for (const jeu of gameResources) {
      for (const r of jeu.recommendations) {
        const bornes = bornesRam(r.ram_gb);
        expect(bornes.length, `${jeu.slug} ${r.ram_gb}`).toBeGreaterThan(0);
        for (const v of bornes) {
          expect(Number.isFinite(v), `${jeu.slug} ${r.ram_gb}`).toBe(true);
          expect(v, `${jeu.slug} ${r.ram_gb}`).toBeGreaterThanOrEqual(MIN_RAM_GO);
          expect(v, `${jeu.slug} ${r.ram_gb}`).toBeLessThanOrEqual(MAX_RAM_GO);
        }
      }
    }
  });

  it("tient le quota vCPU dans les bornes de l'agent", () => {
    for (const jeu of gameResources) {
      for (const r of jeu.recommendations) {
        const v = Number(r.vcpu.replace(",", "."));
        expect(Number.isFinite(v), `${jeu.slug} ${r.vcpu}`).toBe(true);
        expect(v, `${jeu.slug} ${r.vcpu}`).toBeGreaterThanOrEqual(MIN_VCPU_AGENT);
        expect(v, `${jeu.slug} ${r.vcpu}`).toBeLessThanOrEqual(MAX_VCPU_AGENT);
      }
    }
  });

  it("reste sous le quota par defaut du formulaire pour la plupart des paliers", () => {
    // Le curseur de la page de creation va de 0,5 a 6. Une recommandation
    // au-dessus serait inatteignable a la saisie.
    for (const jeu of gameResources) {
      for (const r of jeu.recommendations) {
        expect(
          Number(r.vcpu),
          `${jeu.slug} ${r.players} joueurs : ${r.vcpu} vCPU depasse le curseur`,
        ).toBeLessThanOrEqual(6);
      }
    }
  });

  it("justifie chaque palier par une note", () => {
    // Sans la note, le tableau se lit comme une garantie chiffree alors qu'il
    // decrit un contexte (vanilla, mods, monde neuf...).
    for (const jeu of gameResources) {
      for (const r of jeu.recommendations) {
        expect(r.notes.trim(), `${jeu.slug} ${r.players}`).not.toBe("");
      }
    }
  });
});

describe("trouverParSlug", () => {
  it("retrouve un jeu du catalogue", () => {
    expect(trouverParSlug("palworld")?.name).toBe("Palworld");
  });

  it("renvoie undefined pour un slug inconnu", () => {
    // C'est ce qui declenche le repli sur le catalogue complet dans le guide.
    expect(trouverParSlug("jeu-inexistant")).toBeUndefined();
  });

  it("ne confond pas un slug avec un prefixe d'un autre", () => {
    expect(trouverParSlug("minecraft")).toBeUndefined();
  });
});
