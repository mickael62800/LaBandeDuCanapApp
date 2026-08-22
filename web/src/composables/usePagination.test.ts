import { describe, expect, it } from "vitest";
import { ref } from "vue";
import { usePagination } from "./usePagination";

// Chaque test construit SA propre liste : un état partagé muté par un test
// précédent fausserait les suivants (ordre d'exécution = contamination).
function makeItems(n: number) {
  return ref(Array.from({ length: n }, (_, i) => `item-${i}`));
}

describe("usePagination", () => {
  it("expose la page courante et le nombre total", () => {
    const p = usePagination(makeItems(57), 25);
    expect(p.currentPage.value).toBe(1);
    expect(p.perPage.value).toBe(25);
    expect(p.totalItems.value).toBe(57);
    expect(p.totalPages.value).toBe(Math.ceil(57 / 25)); // 3 pages
  });

  it("retourne la tranche de la page courante", () => {
    const p = usePagination(makeItems(57), 25);
    expect(p.paginatedItems.value).toHaveLength(25);
    expect(p.paginatedItems.value[0]).toBe("item-0");
    p.goToPage(3); // 57 items : la page 3 contient les 7 derniers (indices 50..56)
    expect([...p.paginatedItems.value]).toEqual(["item-50", "item-51", "item-52", "item-53", "item-54", "item-55", "item-56"]);
  });

  it("refuse de sortir des bornes avec goToPage / nextPage / prevPage", () => {
    const p = usePagination(makeItems(57), 25);
    p.goToPage(99); // hors limites : ignoré
    expect(p.currentPage.value).toBe(1);
    p.nextPage();
    expect(p.currentPage.value).toBe(2);
    p.prevPage();
    expect(p.currentPage.value).toBe(1);
  });

  it("accepte la page limite haute", () => {
    const p = usePagination(makeItems(57), 25);
    p.goToPage(3);
    expect(p.currentPage.value).toBe(3);
    p.nextPage(); // pas de page 4
    expect(p.currentPage.value).toBe(3);
  });

  it("recalcule quand la liste change et revient en page 1 si hors bornes", async () => {
    const items = makeItems(57);
    const p = usePagination(items, 25);
    p.goToPage(3);
    items.value = ["a"]; // plus qu'une page possible

    await new Promise((r) => setTimeout(r)); // laisse le watch s'exécuter (microtask + tick)
    expect(p.totalPages.value).toBe(1);
    expect(p.currentPage.value).toBeLessThanOrEqual(1);
  });

  it("gère une liste vide sans page nulle", () => {
    const p = usePagination(makeItems(0), 25);
    expect(p.totalItems.value).toBe(0);
    expect(p.totalPages.value).toBe(1); // jamais 0 : on garde une page affichable
    expect([...p.paginatedItems.value]).toEqual([]);
  });

  it("honore le perPage par défaut fourni", () => {
    const p = usePagination(makeItems(57), 10);
    expect(p.perPage.value).toBe(10);
    expect(p.totalPages.value).toBe(Math.ceil(57 / 10)); // 6
  });

  it("change de perPage à chaud", () => {
    const p = usePagination(makeItems(57), 25);
    p.perPage.value = 20;
    expect(p.paginatedItems.value).toHaveLength(20);
  });
});
