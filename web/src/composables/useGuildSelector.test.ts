import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const getAllMock = vi.hoisted(() => vi.fn());
vi.mock("@/services/guildsService", () => ({
  guildsService: { getAll: (...a: unknown[]) => getAllMock(...(a as [])) },
}));

import type { Guild } from "@/types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

function guilde(id: string): Guild {
  return { guild_id: id, name: `Serveur ${id}` } as unknown as Guild;
}

describe("useGuildSelector", () => {
  beforeEach(() => {
    setActivePinia(createPinia()); // store frais à chaque test (singleton Pinia)
    localStorage.clear();
    getAllMock.mockReset().mockResolvedValue([guilde("g1"), guilde("g2")]);
    useToast().toasts.value = []; // le toast est un singleton module : on nettoie entre tests
  });

  it("expose les refs du store (état initial)", () => {
    const g = useGuildSelector();
    expect(g.guilds.value).toEqual([]);
    expect(g.selectedGuildId.value).toBeNull();
    expect(g.selectedGuild.value).toBeNull();
    expect(g.guildIdFilter.value).toBeUndefined();
    expect(g.loading.value).toBe(false);
  });

  it("fetchGuilds : remplit les refs via le store", async () => {
    const g = useGuildSelector();
    await g.fetchGuilds();
    expect(getAllMock).toHaveBeenCalledTimes(1);
    expect(g.guilds.value.map((x) => x.guild_id)).toEqual(["g1", "g2"]);

    // Les refs restent réactives : une sélection se reflète partout.
    g.selectGuild("g2");
    expect(g.selectedGuildId.value).toBe("g2");
    expect(g.selectedGuild.value?.guild_id).toBe("g2");
    expect(g.guildIdFilter.value).toBe("g2"); // filtre prêt pour les requêtes API
  });

  it("fetchGuilds en échec : toast d'erreur affiché", async () => {
    getAllMock.mockRejectedValueOnce(new Error("503"));
    const g = useGuildSelector();
    await g.fetchGuilds(); // ne doit pas lever (le store catche)

    const toasts = useToast().toasts.value;
    expect(toasts).toHaveLength(1);
    expect(toasts[0].type).toBe("error");
    expect(toasts[0].message).toContain("serveurs");
  });

  it("fetchGuilds en succès : aucun toast", async () => {
    const g = useGuildSelector();
    await g.fetchGuilds();
    expect(useToast().toasts.value).toHaveLength(0);
  });

  it("selectGuild délègue au store (persistance incluse)", async () => {
    const g = useGuildSelector();
    await g.fetchGuilds();
    g.selectGuild("g2");
    expect(localStorage.getItem("sentinel_selected_guild")).toBe("g2");
    g.selectGuild(null);
    expect(g.selectedGuildId.value).toBeNull();
  });
});
