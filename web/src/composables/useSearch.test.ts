import { describe, expect, it } from "vitest";
import { ref } from "vue";
import { useSearch } from "./useSearch";

interface Membre {
  id: string;
  username: string;
  role?: string | null;
}

const membres = ref<Membre[]>([
  { id: "1", username: "Alice", role: "Modo" },
  { id: "2", username: "bob", role: null },
  { id: "3", username: "Charlie", role: "Fondateur" },
]);

describe("useSearch", () => {
  it("retourne tout quand la requête est vide ou des espaces", () => {
    const { search, filtered } = useSearch(membres, ["username"]);
    expect(filtered.value).toHaveLength(3);
    search.value = "   ";
    expect(filtered.value).toHaveLength(3);
  });

  it("filtre insensiblement à la casse sur un champ simple", () => {
    const { search, filtered } = useSearch(membres, ["username"]);
    search.value = "ALIC";
    expect(filtered.value.map((m) => m.id)).toEqual(["1"]);
  });

  it("teste plusieurs champs et les valeurs nulles sans planter", () => {
    const { search, filtered } = useSearch(membres, ["username", "role"]);
    search.value = "modo"; // matche le role de Alice ET pas bob (null)
    expect(filtered.value.map((m) => m.id)).toEqual(["1"]);
  });

  it("supporte un champ fonctionnel", () => {
    const { search, filtered } = useSearch(membres, [(m: Membre) => `${m.username} ${m.role ?? ""}`]);
    search.value = "fondateur"; // uniquement via la concaténation du role
    expect(filtered.value.map((m) => m.id)).toEqual(["3"]);
  });

  it("ne matche rien quand aucun champ ne contient la requête", () => {
    const { search, filtered } = useSearch(membres, ["username", "role"]);
    search.value = "zzz";
    expect(filtered.value).toHaveLength(0);
  });

  it("suit les changements de la liste source (réactif)", () => {
    const { search, filtered } = useSearch(membres, ["username"]);
    search.value = "bob";
    expect(filtered.value.map((m) => m.id)).toEqual(["2"]);
    membres.value.push({ id: "4", username: "Bobby" });
    expect(filtered.value.map((m) => m.id).sort()).toEqual(["2", "4"]);
  });

  it("gère une liste vide en amont", () => {
    const vides = ref<Membre[]>([]);
    const { search, filtered } = useSearch(vides, ["username"]);
    search.value = "x";
    expect(filtered.value).toEqual([]);
  });

  it("ignore les valeurs undefined du champ (coalescence)", () => {
    const avecUndefined = ref<Membre[]>([
      { id: "a", username: "Zoé" },
      // `role` volontairement absent : le champ doit être traité comme vide.
      { id: "b", username: "Yves" },
    ]);
    const { search, filtered } = useSearch(avecUndefined, ["username"]);
    expect(filtered.value).toHaveLength(2); // pas de crash sur l'entrée b
  });
});
