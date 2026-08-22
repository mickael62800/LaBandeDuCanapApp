import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { guildStructureService } from "./guildStructureService";

describe("guildStructureService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getStructure lit l'arborescence du serveur en direct", async () => {
    const structure = [{ id: "c1", name: "general" }];
    mocks.httpGet.mockResolvedValue(structure);

    await expect(guildStructureService.getStructure("g1")).resolves.toEqual(
      structure,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guild-structure/g1");
  });

  it("getRoles lit les roles du serveur en direct", async () => {
    const roles = [{ id: "r1", name: "Membre" }];
    mocks.httpGet.mockResolvedValue(roles);

    await expect(guildStructureService.getRoles("g1")).resolves.toEqual(
      roles,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/guild-structure/g1/roles");
  });

  it("apply envoie le plan complet au serveur", async () => {
    const items = [
      { key: "k1", name: "annonces", kind: "announcement" as const },
      { key: "k2", name: "salle-voix", kind: "voice" as const, parent_key: "k1" },
    ];
    mocks.httpPost.mockResolvedValue({ created: 2, failed: 0, skipped: 0, results: [] });

    await expect(guildStructureService.apply("g7", items)).resolves.toEqual(
      { created: 2, failed: 0, skipped: 0, results: [] },
    );
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/guild-structure/g7/apply", {
      items,
    });
  });

  it("removeChannel supprime le salon cible du serveur", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await guildStructureService.removeChannel("g1", "c9");
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/guild-structure/g1/channels/c9",
    );
  });
});
