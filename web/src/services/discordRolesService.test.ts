import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPatch: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { discordRolesService } from "./discordRolesService";

const roleSynchronise = { id: "r1", name: "Membre", color: 0, position: 1, managed: false };
const roleLive = { id: "r2", name: "Vip", color: 5, position: 3, managed: true };

describe("discordRolesService", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue([]);
  });

  afterEach(async () => {
    // Fait expirer les entrees cachees pour que chaque test parte d'un etat propre.
    await vi.advanceTimersByTimeAsync(3500);
    vi.useRealTimers();
  });

  it("getAll renvoie les roles synchronises quand la table est remplie", async () => {
    mocks.httpGet.mockResolvedValue([roleSynchronise]);

    const resultats = await discordRolesService.getAll("g1");

    expect(resultats).toEqual([roleSynchronise]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/discord-roles/g1");
  });

  it("partage la promesse en cours entre les selecteurs montes dans le meme ecran", async () => {
    mocks.httpGet.mockResolvedValue([roleSynchronise]);

    const premier = discordRolesService.getAll("g1");
    const second = discordRolesService.getAll("g1");
    await Promise.all([premier, second]);

    expect(mocks.httpGet).toHaveBeenCalledTimes(1);
  });

  it("tombe sur l'API live de Discord quand la table est vide et complete le contrat", async () => {
    mocks.httpGet.mockResolvedValueOnce([]).mockResolvedValueOnce([roleLive]);

    const resultats = await discordRolesService.getAll("g9");

    expect(resultats).toEqual([
      {
        ...roleLive,
        guild_id: "g9",
        permissions: "0",
        mentionable: false,
        icon: null,
        member_count: 0,
        synced_at: "",
      },
    ]);
    expect(mocks.httpGet).toHaveBeenNthCalledWith(1, "/api/discord-roles/g9");
    expect(mocks.httpGet).toHaveBeenNthCalledWith(2, "/api/guild-structure/g9/roles");
  });

  it("reessaye la lecture quand l'appel precedent a echoue", async () => {
    mocks.httpGet.mockRejectedValueOnce(new Error("hors ligne"));
    await expect(discordRolesService.getAll("g1")).rejects.toThrow("hors ligne");

    mocks.httpGet.mockResolvedValue([roleSynchronise]);
    const resultats = await discordRolesService.getAll("g1");

    expect(resultats).toEqual([roleSynchronise]);
  });

  it("invalider force la relecture de la liste", async () => {
    mocks.httpGet.mockResolvedValueOnce([roleSynchronise]).mockResolvedValueOnce([{ ...roleSynchronise, name: "Nouveau" }]);

    const avant = await discordRolesService.getAll("g1");
    discordRolesService.invalider("g1");
    const apres = await discordRolesService.getAll("g1");

    expect(avant[0].name).toBe("Membre");
    expect(apres[0].name).toBe("Nouveau");
  });

  it("create enregistre le role et invalide la liste cachee", async () => {
    mocks.httpGet.mockResolvedValue([roleSynchronise]);
    await discordRolesService.getAll("g1");

    const cree = await discordRolesService.create("g1", { name: "Modo", color: 9 });

    expect(mocks.httpPost).toHaveBeenCalledWith("/api/discord-roles/g1/create", {
      name: "Modo",
      color: 9,
    });
    mocks.httpGet.mockClear();
    await discordRolesService.getAll("g1");
    expect(mocks.httpGet).toHaveBeenCalledTimes(1);
    void cree;
  });

  it("edit modifie le role et invalide la liste cachee", async () => {
    mocks.httpGet.mockResolvedValue([roleSynchronise]);
    await discordRolesService.getAll("g1");

    await discordRolesService.edit("g1", "r7", { name: "Renomme" });

    expect(mocks.httpPatch).toHaveBeenCalledWith("/api/discord-roles/g1/r7", {
      name: "Renomme",
    });
  });

  it("remove supprime le role et invalide la liste cachee", async () => {
    mocks.httpGet.mockResolvedValue([roleSynchronise]);
    await discordRolesService.getAll("g1");

    await discordRolesService.remove("g1", "r7");

    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/discord-roles/g1/r7");
  });
});
