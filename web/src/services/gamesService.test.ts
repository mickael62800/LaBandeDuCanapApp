import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { gamesService } from "./gamesService";

describe("gamesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("wallet interroge le portefeuille du joueur session", async () => {
    const wallet = { username: "micka", coins: 120, can_spin: true };
    mocks.httpGet.mockResolvedValue(wallet);

    await expect(gamesService.wallet()).resolves.toBe(wallet);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/me/games/wallet");
  });

  it("history utilise le plafond par defaut puis explicite", async () => {
    await gamesService.history();
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/me/games/history?limit=15",
    );

    await gamesService.history(30);
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/me/games/history?limit=30",
    );
  });

  it("leaderboard utilise le plafond par defaut puis explicite", async () => {
    await gamesService.leaderboard();
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/me/games/leaderboard?limit=10",
    );

    await gamesService.leaderboard(5);
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/me/games/leaderboard?limit=5",
    );
  });

  it("wheelCases lit les cases du serveur pour dessiner la roue", async () => {
    const result = {
      cases: [{ key: "win10", label: "+10", payout: 10, weight: 3 }],
      customized: true,
    };
    mocks.httpGet.mockResolvedValue(result);

    await expect(gamesService.wheelCases()).resolves.toBe(result);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/me/games/wheel/cases");
  });

  it("spinWheel tire via POST avec un corps vide", async () => {
    const spin = { case_key: "win10", payout: 10, balance_after: 230 };
    mocks.httpPost.mockResolvedValue(spin);

    await expect(gamesService.spinWheel()).resolves.toBe(spin);
    expect(mocks.httpPost).toHaveBeenCalledWith(
      "/api/me/games/wheel/spin",
      {},
    );
  });

  it("coussin recupere le dossier complet du joueur en une requete", async () => {
    const file = { profile: {} as never, items: [], combats: [], ranking: [] };
    mocks.httpGet.mockResolvedValue(file);

    await expect(gamesService.coussin()).resolves.toBe(file);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/me/games/coussin");
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("403");
    mocks.httpPost.mockRejectedValue(erreur);
    await expect(gamesService.spinWheel()).rejects.toBe(erreur);
  });
});
