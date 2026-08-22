import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const getMembersMock = vi.hoisted(() => vi.fn());
vi.mock("@/services/guildsService", () => ({
  guildsService: {
    getAll: vi.fn(),
    getMembers: (...a: unknown[]) => getMembersMock(...(a as [])),
  },
}));

import type { GuildMember } from "@/types";

function membre(id: string, username: string, display_name?: string | null): GuildMember {
  return { id, username, display_name: display_name ?? undefined } as unknown as GuildMember;
}

/** Monde isolé : refs module-level du composable sont fraîches à chaque test. */
async function monde() {
  vi.resetModules(); // ré-instantie les modules (refs singleton) pour l'import dynamique suivant
  setActivePinia(createPinia());
  localStorage.clear();
  const mod = await import("./useGuildMembers");
  const storeMod = await import("../stores/guildSelectorStore");
  return { use: () => mod.useGuildMembers(), store: storeMod.useGuildSelectorStore() };
}

describe("useGuildMembers", () => {
  beforeEach(() => {
    getMembersMock.mockReset().mockResolvedValue([membre("100", "alice")]);
  });

  it("sans guilde sélectionnée : aucun appel API, liste vide", async () => {
    const m = await monde(); // aucune guilde choisie dans le store frais
    const g = m.use();
    expect(getMembersMock).not.toHaveBeenCalled();
    expect(g.members.value).toEqual([]);
    expect(g.loading.value).toBe(false);

    g.fetchMembers(); // no-op explicite aussi
    await Promise.resolve();
    expect(getMembersMock).not.toHaveBeenCalled();
  });

  it("chargement des membres de la guilde courante", async () => {
    const m = await monde();
    getMembersMock.mockResolvedValue([membre("100", "alice"), membre("200", "bob")]);
    m.store.selectGuild("g-42"); // déclenche le watch immédiat du composable…

    const g = m.use();
    await vi.waitFor(() => expect(g.loading.value).toBe(false));
    expect(getMembersMock).toHaveBeenCalledWith("g-42");
    expect(g.members.value.map((x) => x.id)).toEqual(["100", "200"]);
  });

  it("recherche : username, display_name et id ; plafonnée à 10", async () => {
    const m = await monde();
    getMembersMock.mockResolvedValue([
      membre("100", "alice"),
      membre("200", "bob", "Alice Wonder"), // match via display_name seulement
      membre("300xyz", "carol"),             // match via id seulement ("300")
    ]);
    m.store.selectGuild("g-42");

    const g = m.use();
    await vi.waitFor(() => expect(g.members.value).toHaveLength(3));

    expect(g.searchMembers("").length).toBe(0); // requête vide -> rien
    expect(g.searchMembers("ali").map((x) => x.id)).toEqual(["100", "200"]); // alice + Alice Wonder (insensible à la casse)
    expect(g.searchMembers("300").map((x) => x.id)).toEqual(["300xyz"]);     // id

    const dixPlus = Array.from({ length: 12 }, (_, i) => membre(String(i), `xx${i}`));
    g.members.value = dixPlus;
    expect(g.searchMembers("xx").length).toBe(10); // slice(0, 10)
  });

  it("échec API : toast + console.error ; la re-tentative est possible", async () => {
    const m = await monde();
    getMembersMock.mockRejectedValueOnce(new Error("503"));
    const spyErr = vi.spyOn(console, "error").mockImplementation(() => {});

    m.store.selectGuild("g-42");
    const g = m.use();
    await vi.waitFor(() => expect(g.loading.value).toBe(false));

    expect(spyErr).toHaveBeenCalled(); // console.error branché sur l'échec
    const { useToast } = await import("./useToast"); // même registre de modules que le composable
    expect(useToast().toasts.value.some((t) => t.type === "error")).toBe(true);

    // loaded n'a PAS été posé : un nouvel essai repart sur l'API.
    getMembersMock.mockResolvedValueOnce([membre("1", "ok")]);
    g.fetchMembers();
    await vi.waitFor(() => expect(g.members.value).toHaveLength(1));
  });

  it("changement de guilde : liste réinitialisée puis rechargée", async () => {
    const m = await monde();
    getMembersMock.mockResolvedValue([membre("100", "alice")]);
    m.store.selectGuild("g-42");

    const g = m.use();
    await vi.waitFor(() => expect(g.members.value).toHaveLength(1));

    // On bascule de guilde : le watch réinitialise et re-fetch.
    getMembersMock.mockClear().mockResolvedValueOnce([membre("900", "zoe")]);
    m.store.selectGuild("g-77");
    await vi.waitFor(() => expect(g.members.value.map((x) => x.id)).toEqual(["900"]));
    expect(getMembersMock).toHaveBeenLastCalledWith("g-77");
  });

  it("fetch déjà chargé : pas de second appel API", async () => {
    const m = await monde();
    getMembersMock.mockResolvedValue([membre("100", "alice")]);
    m.store.selectGuild("g-42");

    const g = m.use();
    await vi.waitFor(() => expect(g.members.value).toHaveLength(1));
    expect(getMembersMock).toHaveBeenCalledTimes(1);

    getMembersMock.mockClear();
    g.fetchMembers(); // loaded=true -> court-circuité
    await Promise.resolve();
    expect(getMembersMock).not.toHaveBeenCalled();
  });
});
