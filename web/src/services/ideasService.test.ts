import { beforeEach, describe, expect, it, vi } from "vitest";

const { httpGet } = vi.hoisted(() => ({ httpGet: vi.fn() }));

vi.mock("@/api/http", () => ({
  httpDelete: vi.fn(),
  httpGet,
  httpPatch: vi.fn(),
}));

import { ideasService } from "./ideasService";

describe("ideasService.list", () => {
  beforeEach(() => {
    httpGet.mockReset();
    httpGet.mockResolvedValue([]);
  });

  it("appelle la collection sans slash final", async () => {
    await ideasService.list();

    expect(httpGet).toHaveBeenCalledWith("/api/ideas");
  });

  it("ajoute les filtres directement apres le chemin de collection", async () => {
    await ideasService.list({
      guild_id: "1510253611572007032",
      status: "nouvelle",
      limit: 50,
      offset: 0,
    });

    expect(httpGet).toHaveBeenCalledWith(
      "/api/ideas?guild_id=1510253611572007032&status=nouvelle&limit=50&offset=0",
    );
  });
});
