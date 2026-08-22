import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";

import GameResourcesGuide from "./GameResourcesGuide.vue";
import { gameResources, trouverParSlug } from "@/data/gameResources";

function monter(props: Record<string, unknown> = {}) {
  return mount(GameResourcesGuide, { props });
}

/// Les jeux effectivement rendus, dans l'ordre.
function jeuxRendus(wrapper: ReturnType<typeof monter>): string[] {
  return wrapper.findAll(".rg-game-name").map((n) => n.text());
}

describe("sans slug : catalogue complet", () => {
  it("affiche tous les jeux du catalogue", () => {
    const wrapper = monter();
    expect(jeuxRendus(wrapper)).toEqual(gameResources.map((g) => g.name));
  });

  it("rend chaque section repliable", () => {
    // Quatorze tableaux ouverts d'affilee sont illisibles : la page de
    // documentation doit pouvoir en refermer.
    const wrapper = monter();
    expect(wrapper.findAll("details.rg-game")).toHaveLength(gameResources.length);
  });

  it("ne nomme aucun jeu dans le titre", () => {
    expect(monter().find(".rg-title").text()).not.toContain("—");
  });
});

describe("avec un slug connu : fiche du seul jeu choisi", () => {
  it("n'affiche que ce jeu", () => {
    // Le defaut d'origine : choisir Palworld affichait les quatorze jeux, et
    // l'utilisateur devait chercher sa ligne au moment de regler la memoire.
    const wrapper = monter({ slug: "palworld" });
    expect(jeuxRendus(wrapper)).toEqual(["Palworld"]);
  });

  it("nomme le jeu dans le titre", () => {
    expect(monter({ slug: "valheim" }).find(".rg-title").text()).toContain("Valheim");
  });

  it("ne rend plus la section repliable", () => {
    // Un seul jeu : le replier ne masquerait plus rien d'autre, et un triangle
    // de repli inviterait a chercher un contenu qui n'existe pas.
    const wrapper = monter({ slug: "palworld" });
    expect(wrapper.findAll("details.rg-game")).toHaveLength(0);
    expect(wrapper.findAll(".rg-game")).toHaveLength(1);
  });

  it("n'avertit de rien", () => {
    expect(monter({ slug: "palworld" }).find(".rg-warn").exists()).toBe(false);
  });

  it("rend exactement les paliers de ce jeu", () => {
    const wrapper = monter({ slug: "terraria" });
    const attendu = trouverParSlug("terraria")!.recommendations;
    const lignes = wrapper.findAll(".rg-table tbody tr");
    expect(lignes).toHaveLength(attendu.length);
    lignes.forEach((tr, i) => {
      const cellules = tr.findAll("td").map((td) => td.text());
      expect(cellules[0]).toBe(String(attendu[i]!.players));
      expect(cellules[1]).toBe(`${attendu[i]!.ram_gb} Go`);
      expect(cellules[2]).toBe(attendu[i]!.vcpu);
      expect(cellules[3]).toBe(attendu[i]!.notes);
    });
  });

  it("fonctionne pour chaque jeu du catalogue", () => {
    // Garantit qu'aucune fiche n'est inatteignable depuis la page de creation.
    for (const jeu of gameResources) {
      expect(jeuxRendus(monter({ slug: jeu.slug })), jeu.slug).toEqual([jeu.name]);
    }
  });
});

describe("avec un slug inconnu : repli signale", () => {
  it("retombe sur le catalogue complet plutot que sur un panneau vide", () => {
    // Un jeu ajoute en base sans fiche ici ne doit pas laisser croire
    // qu'aucune recommandation n'existe.
    const wrapper = monter({ slug: "jeu-pas-encore-documente" });
    expect(jeuxRendus(wrapper)).toEqual(gameResources.map((g) => g.name));
  });

  it("dit pourquoi il montre tout", () => {
    const wrapper = monter({ slug: "jeu-pas-encore-documente" });
    expect(wrapper.find(".rg-warn").exists()).toBe(true);
  });

  it("ne nomme aucun jeu dans le titre", () => {
    const wrapper = monter({ slug: "jeu-pas-encore-documente" });
    expect(wrapper.find(".rg-title").text()).not.toContain("—");
  });
});

describe("unite du reglage processeur", () => {
  it("intitule la colonne vCPU et non « coeurs »", () => {
    // Le champ part en `cpu_limit` -> nano-CPUs Docker : un quota compte en
    // processeurs LOGIQUES. Annoncer des coeurs physiques ferait allouer le
    // double de ce que l'exploitant croit.
    const entetes = monter({ slug: "palworld" })
      .findAll(".rg-table th")
      .map((th) => th.text());
    expect(entetes).toContain("vCPU");
    expect(entetes).not.toContain("Cœurs");
  });

  it("explique l'equivalence threads / coeurs", () => {
    const texte = monter({ slug: "palworld" }).find(".rg-unit").text();
    expect(texte).toContain("threads");
    expect(texte).toContain("cœurs");
  });
});
