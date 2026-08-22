import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  nexusGet: vi.fn(),
  nexusPost: vi.fn(),
  nexusDelete: vi.fn(),
}));

vi.mock("@/api/nexusHttp", () => mocks);

import { nexusMentionableGamesService } from "./nexusMentionableGamesService";

describe("nexusMentionableGamesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("listGames encode la guilde dans le chemin et l'en-tete", async () => {
    const jeux = [{ id: "j1", guild_id: "g/1" }];
    mocks.nexusGet.mockResolvedValue(jeux);

    await expect(nexusMentionableGamesService.listGames("g/1")).resolves.toBe(
      jeux,
    );
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/games/g%2F1", "g/1");
  });

  it("createGame envoie le dto au bot avec la guilde ciblee", async () => {
    const cree = { id: "j9" };
    mocks.nexusPost.mockResolvedValue(cree);
    const dto = { guild_id: "g2", game_name: "Poker", created_by: "u1" };

    await expect(nexusMentionableGamesService.createGame("g2", dto)).resolves.toBe(
      cree,
    );
    expect(mocks.nexusPost).toHaveBeenCalledWith("/api/games", "g2", dto);
  });

  it("deleteGame encode guilde et jeu dans le chemin + acteur en query", async () => {
    await nexusMentionableGamesService.deleteGame("g/1", "jéu", "a&cteur");

    expect(mocks.nexusDelete).toHaveBeenCalledWith(
      "/api/games/g%2F1/j%C3%A9u?actor_id=a%26cteur",
      "g/1",
    );
  });

  it("deployPanel publie le panneau dans le canal choisi", async () => {
    await nexusMentionableGamesService.deployPanel("g3", { channel_id: "c9" });

    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/g3/panel/deploy",
      "g3",
      { channel_id: "c9" },
    );
  });

  it("getSyncReport lit le rapport de divergence", async () => {
    const rapport = { inventory_taken_at: null, divergences: [] };
    mocks.nexusGet.mockResolvedValue(rapport);

    await expect(
      nexusMentionableGamesService.getSyncReport("g4"),
    ).resolves.toBe(rapport);
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/games/g4/sync", "g4");
  });

  it("requestSyncCheck lance une photographie fraiche (202)", async () => {
    await nexusMentionableGamesService.requestSyncCheck("g5");

    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/g5/sync/check",
      "g5",
    );
  });

  it("resolveSync envoie la cle et la direction choisie", async () => {
    const resolution = { key: "k1", applied_now: true };
    mocks.nexusPost.mockResolvedValue(resolution);

    await expect(
      nexusMentionableGamesService.resolveSync("g6", "k1", "discord"),
    ).resolves.toBe(resolution);
    expect(mocks.nexusPost).toHaveBeenCalledWith(
      "/api/games/g6/sync/resolve",
      "g6",
      { key: "k1", direction: "discord" },
    );

    await nexusMentionableGamesService.resolveSync("g6", "k2", "dashboard");
    expect(mocks.nexusPost).toHaveBeenLastCalledWith(
      "/api/games/g6/sync/resolve",
      "g6",
      { key: "k2", direction: "dashboard" },
    );
  });

  it("propage les erreurs du client Nexus", async () => {
    const erreur = new Error("Nexus indisponible");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusMentionableGamesService.listGames("g1")).rejects.toBe(
      erreur,
    );
  });
});
