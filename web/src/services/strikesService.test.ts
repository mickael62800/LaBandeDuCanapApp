import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
  httpPut: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { strikesService } from "./strikesService";

describe("strikesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getConfig lit la configuration des strikes du serveur", async () => {
    const config = { max_strikes: 3 };
    mocks.httpGet.mockResolvedValue(config);

    await expect(strikesService.getConfig("g1")).resolves.toEqual(config);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/strikes/config/g1",
    );
  });

  it("saveConfig ecrit la configuration du serveur", async () => {
    const body = { max_strikes: 5 };
    mocks.httpPut.mockResolvedValue(body);

    await expect(strikesService.saveConfig("g1", body)).resolves.toEqual(
      body,
    );
    expect(mocks.httpPut).toHaveBeenCalledWith("/api/strikes/config/g1", body);
  });

  it("getActiveStrikes lit les strikes actifs d'un membre", async () => {
    const strikes = [{ id: "s1" }];
    mocks.httpGet.mockResolvedValue(strikes);

    await expect(
      strikesService.getActiveStrikes("g1", "u2"),
    ).resolves.toEqual([{ id: "s1" }]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/strikes/g1/u2");
  });

  it("addStrike enregistre un nouveau strike", async () => {
    const body = { guild_id: "g1", user_id: "u2", reason: "spam" };
    mocks.httpPost.mockResolvedValue({ count: 1, action: null });

    await expect(strikesService.addStrike(body)).resolves.toEqual({
      count: 1,
      action: null,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/strikes", body);
  });

  it("resetStrikes efface les strikes d'un membre", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await strikesService.resetStrikes("g1", "u2");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/strikes/g1/u2");
  });
});
