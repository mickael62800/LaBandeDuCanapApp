import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { analyticsService } from "./analyticsService";

describe("analyticsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getFull sans serveur utilise la periode par defaut de 30 jours", async () => {
    const analytics = { total: 1 };
    mocks.httpGet.mockResolvedValue(analytics);

    await expect(analyticsService.getFull()).resolves.toEqual(analytics);
    // guild_id absent (null) : seul `days` est serialise.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/analytics?days=30");
  });

  it("getFull cible un serveur et une periode explicites", async () => {
    mocks.httpGet.mockResolvedValue({ total: 2 });

    await analyticsService.getFull("g1", 7);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/analytics?guild_id=g1&days=7",
    );
  });

  it("reset efface les lignes d'analytics du serveur", async () => {
    mocks.httpPost.mockResolvedValue({ deleted_rows: 42 });

    await expect(analyticsService.reset("g1")).resolves.toEqual({
      deleted_rows: 42,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith(
      "/api/analytics/reset?guild_id=g1",
      {},
    );
  });
});
