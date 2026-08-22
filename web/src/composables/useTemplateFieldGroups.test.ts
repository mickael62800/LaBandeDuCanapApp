import { describe, expect, it } from "vitest";
import { ref } from "vue";
import type { TemplateField } from "@/services/nexusGamesService";
import { SECTION_PAR_DEFAUT, useTemplateFieldGroups } from "./useTemplateFieldGroups";

function champ(key: string, type: TemplateField["type"], group?: string | null): TemplateField {
  return { key, label: `Label ${key}`, type, group };
}

describe("useTemplateFieldGroups", () => {
  it("regroupe par section et met le fourre-tout en dernier", () => {
    const schema = ref<TemplateField[]>([
      champ("A", "text"), // sans groupe -> défaut
      champ("B", "boolean", "Combat"),
      champ("C", "number"), // sans groupe -> défaut
      champ("D", "enum", "Réseau"),
    ]);
    const groups = useTemplateFieldGroups(schema);

    expect(groups.value.map((g) => g.nom)).toEqual(["Combat", "Réseau", SECTION_PAR_DEFAUT]);
    expect(groups.value[2].champs.map((f) => f.key).sort()).toEqual(["A", "C"]);
  });

  it("trie par nature à l'intérieur d'une section (bool, enum, number, text)", () => {
    const schema = ref<TemplateField[]>([
      champ("T1", "text", "S"),
      champ("N1", "number", "S"),
      champ("E1", "enum", "S"),
      champ("B1", "boolean", "S"),
    ]);
    const groups = useTemplateFieldGroups(schema);
    expect(groups.value[0].champs.map((f) => f.key)).toEqual(["B1", "E1", "N1", "T1"]);
  });

  it("tri stable : à nature égale, l'ordre du schéma est conservé", () => {
    const schema = ref<TemplateField[]>([
      champ("SPAWN_ANIMALS", "boolean", "G"),
      champ("ZULU", "text", "G"),
      champ("SPAWN_MONSTERS", "boolean", "G"),
    ]);
    const groups = useTemplateFieldGroups(schema);
    expect(groups.value[0].champs.map((f) => f.key)).toEqual([
      "SPAWN_ANIMALS", // bool, premier dans le schéma
      "SPAWN_MONSTERS", // bool, second
      "ZULU", // text en dernier
    ]);
  });

  it("groupe vide / null / undefined sans planter", () => {
    expect(useTemplateFieldGroups(ref([])).value).toEqual([]);
    const nul = ref<TemplateField[] | null>(null);
    expect(useTemplateFieldGroups(nul as never).value).toEqual([]);
  });

  it("groupe vide (blancs) retombe sur la section par défaut", () => {
    const schema = ref<TemplateField[]>([champ("X", "text", "   "), champ("Y", "boolean")]);
    const groups = useTemplateFieldGroups(schema);
    expect(groups.value).toHaveLength(1); // les deux dans le fourre-tout
    expect(groups.value[0].nom).toBe(SECTION_PAR_DEFAUT);
  });

  it("suit la réactivité du schéma", () => {
    const schema = ref<TemplateField[]>([]);
    const groups = useTemplateFieldGroups(schema);
    expect(groups.value).toHaveLength(0);
    schema.value = [champ("K1", "boolean", "Sec")];
    expect(groups.value.map((g) => g.nom)).toEqual(["Sec"]);
  });

  it("type inconnu du schéma : trié après les connus, sans crash", () => {
    // Type hors de l'union pour tester la tolérance (`?? 9` dans le tri).
    const exotique = { key: "EXOTIQUE", label: "Exo", type: "slider", group: "S" } as unknown as TemplateField;
    const schema = ref<TemplateField[]>([champ("TEXTE", "text", "S"), exotique]);
    const groups = useTemplateFieldGroups(schema);
    expect(groups.value[0].champs.map((f) => f.key)).toEqual(["TEXTE", "EXOTIQUE"]);
  });
});
