import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const getAllMock = vi.hoisted(() => vi.fn());
vi.mock("@/services/guildsService", () => ({
  guildsService: { getAll: (...a: unknown[]) => getAllMock(...(a as [])) },
}));

// siteConfig est un module d'état global : on le pilote par test.
const configState = vi.hoisted(() => ({ value: "" }));
vi.mock("@/siteConfig", () => ({
  siteConfig: () => (configState.value ? { guildId: configState.value, discordInvite: "" } : { guildId: "", discordInvite: "" }),
}));

import type { Guild } from "@/types";
import { useGuildSelectorStore } from "./guildSelectorStore";

function guilde(id: string): Guild {
  return { guild_id: id, name: `Serveur ${id}` } as unknown as Guild;
}

const CACHE_KEY = "sentinel_guilds_cache";
const SELECTED_KEY = "sentinel_selected_guild";

/** API en suspens : on contrôle le moment exact de la résolution (et sa valeur). */
function apiEnAttente() {
  let resolveAll!: (v: unknown) => void;
  getAllMock.mockReturnValue(new Promise((r) => { resolveAll = r as never; }));
  return (valeurs?: Guild[]) => resolveAll(valeurs ?? [guilde("g1"), guilde("g2")]);
}

describe("guildSelectorStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia()); // store frais à chaque test (defineStore est singleton)
    localStorage.clear();
    configState.value = "";
    getAllMock.mockReset().mockResolvedValue([guilde("g1"), guilde("g2")]);
  });

  it("état initial : vide, pas de sélection", () => {
    const s = useGuildSelectorStore();
    expect(s.guilds).toEqual([]);
    expect(s.selectedGuildId).toBeNull();
    expect(s.selectedGuild).toBeNull();
    expect(s.guildIdFilter).toBeUndefined();
  });

  it("fetch : liste chargée, sélection nulle sans sauvegarde", async () => {
    const s = useGuildSelectorStore();
    await s.fetchGuilds();
    expect(getAllMock).toHaveBeenCalledTimes(1);
    expect(s.guilds.map((g) => g.guild_id)).toEqual(["g1", "g2"]);
    // Aucune guilde sauvegardée : la sélection reste à null (le store ne devine pas).
    expect(s.selectedGuildId).toBeNull();
  });

  it("cache local valide : hydraté AVANT le réseau, puis validé par l'API", async () => {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ data: [guilde("gc")], ts: Date.now() }));
    const settle = apiEnAttente(); // API en suspens pour observer l'état intermédiaire

    const s = useGuildSelectorStore();
    void s.fetchGuilds(); // corps synchrone : hydratation du cache déjà faite…
    expect(s.guilds.map((g) => g.guild_id)).toEqual(["gc"]); // …avant que l'API réponde.
    expect(getAllMock).toHaveBeenCalledTimes(1);

    settle([guilde("f1")]); // réponse fraîche (différente du cache)
    await Promise.resolve(); await Promise.resolve(); // laisse le .then() du store se poser
    expect(s.loading).toBe(false);
    expect(s.guilds.map((g) => g.guild_id)).toEqual(["f1"]); // remplacé par le frais

    // Le cache a été réécrit avec la donnée fraîche.
    const recache = JSON.parse(localStorage.getItem(CACHE_KEY)!);
    expect(recache.data[0].guild_id).toBe("f1");
  });

  it("cache expiré (TTL dépassé) : ignoré", async () => {
    localStorage.setItem(CACHE_KEY, JSON.stringify({ data: [guilde("gc")], ts: Date.now() - 7 * 3600_000 }));
    const settle = apiEnAttente();

    const s = useGuildSelectorStore();
    void s.fetchGuilds();
    expect(s.guilds).toEqual([]); // pas d'hydratation depuis un cache expiré

    settle([guilde("g1")]);
    await vi.waitFor(() => expect(s.loading).toBe(false));
    expect(s.guilds.map((g) => g.guild_id)).toEqual(["g1"]);
  });

  it("cache corrompu : ignoré sans exception", async () => {
    localStorage.setItem(CACHE_KEY, "ceci n'est pas du json");
    const settle = apiEnAttente();

    const s = useGuildSelectorStore();
    void s.fetchGuilds(); // JSON.parse lève -> catché dans loadCache()
    expect(s.guilds).toEqual([]);

    settle([guilde("g1")]);
    await vi.waitFor(() => expect(s.loading).toBe(false));
    expect(s.guilds.map((g) => g.guild_id)).toEqual(["g1"]);
  });

  it("guilde sauvegardée encore dans la liste : restaurée", async () => {
    localStorage.setItem(SELECTED_KEY, "g2");
    const s = useGuildSelectorStore();
    await s.fetchGuilds();
    expect(s.selectedGuildId).toBe("g2");
  });

  it("guilde sauvegardée disparue de la liste : sélection reste nulle", async () => {
    localStorage.setItem(SELECTED_KEY, "ghost"); // ghost ne sera pas dans la réponse API
    const s = useGuildSelectorStore();
    await s.fetchGuilds();
    expect(s.selectedGuildId).toBeNull(); // rien à restaurer -> null
  });

  it("guilde sélectionnée qui disparaît de l'API : reset à null", async () => {
    const s = useGuildSelectorStore();
    s.selectGuild("old"); // choix persisté…
    getAllMock.mockClear().mockResolvedValueOnce([guilde("g1")]); // …mais plus dans la liste fraîche
    await s.fetchGuilds();
    expect(s.selectedGuildId).toBeNull(); // reset défensif
  });

  it("selectGuild : persiste le choix ; null l'efface", async () => {
    const s = useGuildSelectorStore();
    await s.fetchGuilds();

    s.selectGuild("g2");
    expect(s.selectedGuildId).toBe("g2");
    expect(localStorage.getItem(SELECTED_KEY)).toBe("g2");

    s.selectGuild(null);
    expect(s.selectedGuildId).toBeNull();
    expect(localStorage.getItem(SELECTED_KEY)).toBeNull();
  });

  it("selectedGuild : la guilde courante ou null", async () => {
    const s = useGuildSelectorStore();
    await s.fetchGuilds();

    s.selectGuild("g2");
    expect(s.selectedGuild?.guild_id).toBe("g2"); // trouve dans la liste…

    s.selectGuild(null);
    expect(s.selectedGuild).toBeNull(); // …et null une fois désélectionnée.
  });

  it("API en échec : erreur exposée, liste inchangée", async () => {
    getAllMock.mockRejectedValueOnce(new Error("503"));
    const s = useGuildSelectorStore();
    await s.fetchGuilds();
    expect(s.error).toBe("Error: 503");
    expect(s.guilds).toEqual([]);
    expect(s.loading).toBe(false);
  });

  it("localStorage indisponible (quota) : le fetch aboutit quand même", async () => {
    const setItem = localStorage.setItem.bind(localStorage);
    vi.spyOn(Storage.prototype, "setItem").mockImplementation((k: string, v?: string | null) => {
      if (String(k).startsWith("sentinel_")) throw new Error("QuotaExceededError");
      return setItem(k, v ?? "");
    });
    const s = useGuildSelectorStore();
    await expect(s.fetchGuilds()).resolves.toBeUndefined(); // pas d'exception levée
    expect(s.guilds.map((g) => g.guild_id)).toEqual(["g1", "g2"]);
  });

  it("fetch avec liste déjà peuplée : loading reste faux (pas de spinner)", async () => {
    const s = useGuildSelectorStore();
    await s.fetchGuilds();
    expect(s.loading).toBe(false);

    getAllMock.mockClear();
    let resolu = false;
    void s.fetchGuilds().then(() => {
      resolu = true;
    });
    // Pendant le 2e fetch, la liste est déjà remplie -> pas de spinner.
    expect(s.loading).toBe(false);
    await vi.waitFor(() => expect(resolu).toBe(true));
  });

  describe("mode mono-serveur (guildId imposé par la config)", () => {
    beforeEach(() => {
      configState.value = "g-imposee";
    });

    it("la sélection est figée sur la guilde imposée dès le départ", async () => {
      const s = useGuildSelectorStore();
      expect(s.selectedGuildId).toBe("g-imposee"); // positionnée à la création du store

      await s.fetchGuilds();
      expect(s.selectedGuildId).toBe("g-imposee"); // même si l'API ne la connaît pas encore
    });

    it("selectGuild est ignoré silencieusement", async () => {
      const s = useGuildSelectorStore();
      await s.fetchGuilds();
      s.selectGuild("autre");
      expect(s.selectedGuildId).toBe("g-imposee");
      expect(localStorage.getItem(SELECTED_KEY)).toBeNull(); // rien n'est persisté
    });

    it("la guilde sauvegardée ne prime pas sur l'imposée", async () => {
      localStorage.setItem(SELECTED_KEY, "sauvegardee");
      const s = useGuildSelectorStore();
      await s.fetchGuilds();
      expect(s.selectedGuildId).toBe("g-imposee");
    });

    it("hydratation depuis le cache : la sélection reste l'imposée", async () => {
      localStorage.setItem(CACHE_KEY, JSON.stringify({ data: [guilde("gc")], ts: Date.now() }));
      const settle = apiEnAttente();

      const s = useGuildSelectorStore();
      void s.fetchGuilds(); // corps synchrone : hydratation du cache déjà faite…
      expect(s.guilds.map((g) => g.guild_id)).toEqual(["gc"]); // …liste venue du cache,
      expect(s.selectedGuildId).toBe("g-imposee"); // …mais sélection imposée (pas "sauvegardée").

      settle([guilde("f1")]);
      await vi.waitFor(() => expect(s.loading).toBe(false));
    });
  });
});
