import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ nexusGet: vi.fn() }));

vi.mock("@/api/nexusHttp", () => mocks);

import { nexusEconomyService } from "./nexusEconomyService";

describe("nexusEconomyService (portefeuilles Nexus)", () => {
  beforeEach(() => {
    mocks.nexusGet.mockReset().mockResolvedValue([]);
  });

  it("leaderboard lit le classement avec la limite par defaut de 20", async () => {
    const top = [{ guild_id: "g1", user_id: "u1", username: "micka", coins: 999 }];
    mocks.nexusGet.mockResolvedValue(top);

    await expect(nexusEconomyService.leaderboard("g1")).resolves.toBe(top);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/wallet/g1/leaderboard?limit=20",
      "g1",
    );
  });

  it("leaderboard honore une limite explicite et encode la guilde", async () => {
    await nexusEconomyService.leaderboard("g/9", 5);
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/wallet/g%2F9/leaderboard?limit=5",
      "g/9",
    );
  });

  it("wallet lit le portefeuille du couple guilde/utilisateur (encode)", async () => {
    const wallet = { coins: 42, total_earned: 100, total_spent: 58 };
    mocks.nexusGet.mockResolvedValue(wallet);

    await expect(nexusEconomyService.wallet("g/1", "u&2")).resolves.toBe(
      wallet,
    );
    expect(mocks.nexusGet).toHaveBeenCalledWith("/api/wallet/g%2F1/u%262", "g/1");
  });

  it("history lit les transactions avec la limite par defaut de 50", async () => {
    const tx = [{ id: "t1", amount: -10, balance_after: 32 }];
    mocks.nexusGet.mockResolvedValue(tx);

    await expect(nexusEconomyService.history("g7", "u8")).resolves.toBe(tx);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/wallet/g7/u8/history?limit=50",
      "g7",
    );
  });

  it("history honore une limite explicite", async () => {
    await nexusEconomyService.history("g7", "u8", 3);
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/wallet/g7/u8/history?limit=3",
      "g7",
    );
  });

  it("propage les erreurs du client Nexus", async () => {
    const erreur = new Error("401");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusEconomyService.wallet("gX", "uY")).rejects.toBe(erreur);
  });
});
