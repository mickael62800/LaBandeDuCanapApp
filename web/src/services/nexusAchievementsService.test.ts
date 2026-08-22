import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  nexusGet: vi.fn(),
  nexusPatch: vi.fn(),
}));

vi.mock("@/api/nexusHttp", () => mocks);

import { nexusAchievementsService } from "./nexusAchievementsService";

describe("nexusAchievementsService (hauts faits)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list lit le catalogue global sans filtre de jeu", async () => {
    const defs = [{ id: "a1", game: null, code: "first" }];
    mocks.nexusGet.mockResolvedValue(defs);

    await expect(nexusAchievementsService.list("g1")).resolves.toBe(defs);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/achievements/definitions",
      "g1",
    );
  });

  it("list filtre par slug de jeu (encode)", async () => {
    const defs = [{ id: "a2" }];
    mocks.nexusGet.mockResolvedValue(defs);

    await expect(
      nexusAchievementsService.list("g1", "palworld"),
    ).resolves.toBe(defs);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/achievements/definitions?game=palworld",
      "g1",
    );

    // Un slug avec caracteres speciaux doit rester valide dans la query.
    await nexusAchievementsService.list("g2", "jeu/spécial");
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/achievements/definitions?game=jeu%2Fsp%C3%A9cial",
      "g2",
    );
  });

  it("update patche la definition cible (image, nom, drapeaux)", async () => {
    const maj = { id: "a7" };
    mocks.nexusPatch.mockResolvedValue(maj);

    await expect(
      nexusAchievementsService.update("g3", "a7", { icon_url: null }),
    ).resolves.toBe(maj);
    expect(mocks.nexusPatch).toHaveBeenCalledWith(
      "/api/achievements/definitions/a7",
      "g3",
      { icon_url: null },
    );

    // id avec caracteres speciaux : encode dans le chemin.
    await nexusAchievementsService.update("g4", "a/x&y", { enabled: false });
    expect(mocks.nexusPatch).toHaveBeenLastCalledWith(
      "/api/achievements/definitions/a%2Fx%26y",
      "g4",
      { enabled: false },
    );
  });

  it("propage les erreurs du client Nexus", async () => {
    const erreur = new Error("Nexus KO");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusAchievementsService.list("g1")).rejects.toBe(erreur);
  });
});
