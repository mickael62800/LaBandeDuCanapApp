import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { levelsService } from "./levelsService";

describe("levelsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("addXp envoie le gain d'experience du membre", async () => {
    const body = {
      guild_id: "g1",
      user_id: "u2",
      username: "micka",
      amount: 5,
      source: "text" as const,
    };
    mocks.httpPost.mockResolvedValue(undefined);

    await levelsService.addXp(body);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/levels/xp", body);
  });

  it("getLeaderboard lit le classement du serveur", async () => {
    const board = [{ user_id: "u2", xp_text: 10 }];
    mocks.httpGet.mockResolvedValue(board);

    await expect(levelsService.getLeaderboard("g1")).resolves.toEqual([
      { user_id: "u2", xp_text: 10 },
    ]);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/levels/g1/leaderboard",
    );
  });

  it("setUserXp fixe la valeur exacte de l'XP admin", async () => {
    const body = { guild_id: "g1", user_id: "u2", xp_text: 40 };
    mocks.httpPost.mockResolvedValue({ user_id: "u2" });

    await levelsService.setUserXp(body);
    expect(mocks.httpPost).toHaveBeenCalledWith(
      "/api/levels/admin/set-xp",
      body,
    );
  });

  it("resetUserXp remet l'XP a zero sur la cible demandee", async () => {
    const body = { guild_id: "g1", user_id: "u2", target: "voice" as const };
    mocks.httpPost.mockResolvedValue({ user_id: "u2" });

    await levelsService.resetUserXp(body);
    expect(mocks.httpPost).toHaveBeenCalledWith(
      "/api/levels/admin/reset-xp",
      body,
    );
  });
});
