import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { effectScope, type EffectScope } from "vue";
import { createPinia, setActivePinia } from "pinia";

const leaderboardMock = vi.hoisted(() => vi.fn());
const rolesMock = vi.hoisted(() => vi.fn());
vi.mock("@/services/levelsService", () => ({
  levelsService: { getLeaderboard: (...a: unknown[]) => leaderboardMock(...(a as [])) },
}));
vi.mock("@/services/discordRolesService", () => ({
  discordRolesService: { getAll: (...a: unknown[]) => rolesMock(...(a as [])) },
}));

import { useLevels } from "./useLevels";
import { useGuildSelectorStore } from "@/stores/guildSelectorStore";

// Fait avancer la file des microtâches : les services mockés résolvent
// immédiatement, il suffit de laisser Promise.all + .then se résoudre.
async function tick(times = 10) {
  for (let i = 0; i < times; i++) await Promise.resolve();
}

describe("useLevels", () => {
  let scope: EffectScope | null = null;

  beforeEach(() => {
    setActivePinia(createPinia()); // store frais à chaque test (defineStore est singleton)
    leaderboardMock.mockReset().mockResolvedValue([{ user_id: "u1" }]);
    rolesMock.mockReset().mockResolvedValue([{ role_id: "r1" }]);
  });

  afterEach(() => {
    scope?.stop(); // arrête le watch(selectedGuildId) créé par le composable
    scope = null;
  });

  function monter() {
    const s = effectScope(true);
    scope = s;
    return s.run(() => useLevels())!;
  }

  it("sans guilde : listes vides, loading faux, aucun appel service", async () => {
    const store = useGuildSelectorStore();
    store.selectedGuildId = null;

    const l = monter();
    await l.fetchAll(); // onMounted n'existe pas hors composant : on appelle explicitement
    await tick();

    expect(l.leaderboard.value).toEqual([]);
    expect(l.roles.value).toEqual([]);
    expect(l.loading.value).toBe(false);
    expect(leaderboardMock).not.toHaveBeenCalled();
  });

  it("avec guilde : charge le classement et les rôles en parallèle", async () => {
    const store = useGuildSelectorStore();
    store.selectedGuildId = "g1";

    const l = monter();
    await l.fetchAll();
    await tick();

    expect(leaderboardMock).toHaveBeenCalledWith("g1");
    expect(rolesMock).toHaveBeenCalledWith("g1");
    expect(l.leaderboard.value).toEqual([{ user_id: "u1" }]);
    expect(l.roles.value).toEqual([{ role_id: "r1" }]);
    expect(l.error.value).toBeNull();
  });

  it("un service en échec : l'autre passe, listes vides mais pas d'erreur globale", async () => {
    const store = useGuildSelectorStore();
    store.selectedGuildId = "g1";
    leaderboardMock.mockRejectedValueOnce(new Error("503"));

    const l = monter();
    await l.fetchAll();
    await tick();

    expect(l.leaderboard.value).toEqual([]); // .catch(() => [])
    expect(l.roles.value).toEqual([{ role_id: "r1" }]);
    expect(l.error.value).toBeNull();
  });

  it("exception synchrone : erreur affichée + toast", async () => {
    const store = useGuildSelectorStore();
    store.selectedGuildId = "g1";
    leaderboardMock.mockImplementation(() => {
      throw new Error("boom"); // rejet SYNCHRONE -> Promise.all lève avant les .catch internes
    });

    const l = monter();
    // Le throw synchrone est capturé par le try/catch de fetchAll : la promesse se RÉSOUT,
    // avec l'erreur dans error.value (et un toast), pas en rejetant.
    await expect(l.fetchAll()).resolves.toBeUndefined();
    await tick();

    expect(l.error.value).toBe("Impossible de charger les niveaux.");
    expect(l.loading.value).toBe(false);
  });

  it("changer de guilde relance le chargement ; repasser à null vide tout", async () => {
    const store = useGuildSelectorStore();
    store.selectedGuildId = "g1";

    const l = monter();
    await l.fetchAll();
    await tick();
    expect(l.leaderboard.value).toEqual([{ user_id: "u1" }]);

    leaderboardMock.mockClear();
    store.selectedGuildId = "g2"; // watch(selectedGuildId) -> refetch automatique
    await tick(30);
    expect(leaderboardMock).toHaveBeenCalledWith("g2");

    store.selectedGuildId = null;
    await tick(30);
    expect(l.leaderboard.value).toEqual([]);
  });
});
