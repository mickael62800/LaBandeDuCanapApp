import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ nexusGet: vi.fn() }));

vi.mock("@/api/nexusHttp", () => mocks);

import { nexusCoussinService } from "./nexusCoussinService";

describe("nexusCoussinService (supervision du jeu Coussin)", () => {
  beforeEach(() => {
    mocks.nexusGet.mockReset().mockResolvedValue([]);
  });

  it("ranking lit le classement avec la limite par defaut de 50", async () => {
    const top = [{ user_id: "u1", username: "micka", level: 7, total_wins: 3 }];
    mocks.nexusGet.mockResolvedValue(top);

    await expect(nexusCoussinService.ranking("g1")).resolves.toBe(top);
    expect(mocks.nexusGet).toHaveBeenCalledWith(
      "/api/coussin/g1/classement?limit=50",
      "g1",
    );
  });

  it("ranking honore une limite explicite et encode la guilde", async () => {
    await nexusCoussinService.ranking("g/9", 3);
    expect(mocks.nexusGet).toHaveBeenLastCalledWith(
      "/api/coussin/g%2F9/classement?limit=3",
      "g/9",
    );
  });

  it("propage les erreurs du client Nexus (lecture seule, aucune action de jeu)", async () => {
    const erreur = new Error("401");
    mocks.nexusGet.mockRejectedValue(erreur);
    await expect(nexusCoussinService.ranking("gX")).rejects.toBe(erreur);
  });
});
