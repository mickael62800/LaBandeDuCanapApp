import { describe, expect, it } from "vitest";
import { ref } from "vue";

import {
  useTemplateFieldGroups,
  SECTION_PAR_DEFAUT,
} from "./useTemplateFieldGroups";
import type { TemplateField } from "@/services/nexusGamesService";

function champ(over: Partial<TemplateField>): TemplateField {
  return { key: "K", label: "L", type: "text", ...over } as TemplateField;
}

function grouper(champs: TemplateField[]) {
  return useTemplateFieldGroups(ref(champs)).value;
}

describe("regroupement en sections", () => {
  it("rassemble les champs par section du schema", () => {
    const groupes = grouper([
      champ({ key: "A", group: "Monde" }),
      champ({ key: "B", group: "Regles du jeu" }),
      champ({ key: "C", group: "Monde" }),
    ]);
    expect(groupes.map((g) => g.nom)).toEqual(["Monde", "Regles du jeu"]);
    expect(groupes[0]!.champs.map((c) => c.key)).toEqual(["A", "C"]);
  });

  it("verse les champs sans section dans le fourre-tout", () => {
    const groupes = grouper([champ({ key: "A" })]);
    expect(groupes[0]!.nom).toBe(SECTION_PAR_DEFAUT);
  });

  it("traite une section vide ou blanche comme absente", () => {
    // Un `group` a la chaine vide vient d'une migration bâclee : le laisser
    // creerait une section sans nom dans le formulaire.
    const groupes = grouper([champ({ key: "A", group: "   " }), champ({ key: "B" })]);
    expect(groupes).toHaveLength(1);
    expect(groupes[0]!.nom).toBe(SECTION_PAR_DEFAUT);
  });

  it("place le fourre-tout en dernier", () => {
    // Les sections nommees portent une intention, lui n'est qu'un reste.
    const groupes = grouper([
      champ({ key: "A" }),
      champ({ key: "B", group: "Monde" }),
    ]);
    expect(groupes.map((g) => g.nom)).toEqual(["Monde", SECTION_PAR_DEFAUT]);
  });

  it("rend une liste vide pour un schema absent", () => {
    expect(useTemplateFieldGroups(ref(undefined)).value).toEqual([]);
    expect(useTemplateFieldGroups(ref(null)).value).toEqual([]);
  });
});

describe("ordre a l'interieur d'une section", () => {
  it("groupe les interrupteurs en tete, puis listes, nombres et textes", () => {
    // Alterner interrupteurs et saisies obligeait l'oeil a changer de mode a
    // chaque ligne, ce qui donnait l'impression de fouillis.
    const groupes = grouper([
      champ({ key: "TXT", type: "text", group: "S" }),
      champ({ key: "NUM", type: "number", group: "S" }),
      champ({ key: "BOOL", type: "boolean", group: "S" }),
      champ({ key: "ENUM", type: "enum", group: "S" }),
    ]);
    expect(groupes[0]!.champs.map((c) => c.key)).toEqual(["BOOL", "ENUM", "NUM", "TXT"]);
  });

  it("conserve l'ordre du schema entre champs de meme nature", () => {
    // Un tri alphabetique separerait SPAWN_ANIMALS de SPAWN_MONSTERS.
    const groupes = grouper([
      champ({ key: "SPAWN_NPCS", type: "boolean", group: "Monde" }),
      champ({ key: "SPAWN_ANIMALS", type: "boolean", group: "Monde" }),
      champ({ key: "SPAWN_MONSTERS", type: "boolean", group: "Monde" }),
    ]);
    expect(groupes[0]!.champs.map((c) => c.key)).toEqual([
      "SPAWN_NPCS",
      "SPAWN_ANIMALS",
      "SPAWN_MONSTERS",
    ]);
  });

  it("range un type inconnu apres les types connus", () => {
    const groupes = grouper([
      champ({ key: "X", type: "exotique" as TemplateField["type"], group: "S" }),
      champ({ key: "T", type: "text", group: "S" }),
    ]);
    expect(groupes[0]!.champs.map((c) => c.key)).toEqual(["T", "X"]);
  });
});

describe("doublons de cle", () => {
  it("les laisse passer, faute de pouvoir choisir lequel fait foi", () => {
    // Le composable ne dedoublonne pas : deux entrees de meme cle sont un
    // defaut de DONNEES, corrige par la migration 064. Ce test fige le
    // comportement pour que personne ne masque le probleme ici — le formulaire
    // afficherait deux champs ecrivant la meme cle, et le dernier gagnerait.
    const groupes = grouper([
      champ({ key: "ALLOW_NETHER", type: "boolean", group: "Monde", label: "ancien" }),
      champ({ key: "ALLOW_NETHER", type: "boolean", group: "Monde", label: "nouveau" }),
    ]);
    expect(groupes[0]!.champs).toHaveLength(2);
  });
});
