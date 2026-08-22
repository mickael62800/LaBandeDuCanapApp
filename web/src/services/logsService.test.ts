import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { logsService } from "./logsService";

describe("logsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getAll lit la collection sans filtre quand tout est vide", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await logsService.getAll(null, null, "all", null);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/logs");
  });

  it("getAll assemble les filtres serveur, categorie et niveau", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await logsService.getAll("g1", "moderation", "warn", 50);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/logs?guild_id=g1&category=moderation&level=warn&limit=50",
    );
  });

  it("deleteByCategory supprime les journaux d'une categorie", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await logsService.deleteByCategory("system");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/logs/system");
  });
});
