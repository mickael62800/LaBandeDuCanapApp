import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { membersService } from "./membersService";

describe("membersService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getAll lit les membres du serveur", async () => {
    const membres = [{ id: "u1" }];
    mocks.httpGet.mockResolvedValue(membres);

    await expect(membersService.getAll("g1")).resolves.toEqual(
      membres,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/members/g1");
  });

  it("getSummary lit le resume du membre", async () => {
    const resume = { user_id: "u2" };
    mocks.httpGet.mockResolvedValue(resume);

    await expect(membersService.getSummary("g1", "u2")).resolves.toEqual(
      resume,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/members/g1/u2/summary");
  });

  it("resetMember reinitialise le membre et renvoie les totaux supprimes", async () => {
    const resultat = { status: "ok", guild_id: "g1", user_id: "u2", totals: { strikes: 3 } };
    mocks.httpPost.mockResolvedValue(resultat);

    await expect(membersService.resetMember("g1", "u2")).resolves.toEqual(
      resultat,
    );
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/members/g1/u2/reset");
  });
});
