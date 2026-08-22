import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ anonymousJsonGet: vi.fn() }));

vi.mock("./publicHttp", () => mocks);

import { publicGamesService } from "./publicGamesService";

describe("publicGamesService (vitrine publique)", () => {
  beforeEach(() => {
    mocks.anonymousJsonGet.mockReset().mockResolvedValue([]);
  });

  it("listServers interroge la location /nexus-public sans session", async () => {
    const serveurs = [
      { id: "s1", name: "Palworld FR", game: "Palworld", online: true, player_count: 3 },
    ];
    mocks.anonymousJsonGet.mockResolvedValue(serveurs);

    await expect(publicGamesService.listServers("g/1")).resolves.toBe(serveurs);
    expect(mocks.anonymousJsonGet).toHaveBeenCalledWith(
      "/nexus-public/api/public/games/g%2F1/servers",
    );
  });

  it("propage les erreurs du transport public (sans redirection login)", async () => {
    const erreur = new Error("403");
    mocks.anonymousJsonGet.mockRejectedValue(erreur);
    await expect(publicGamesService.listServers("gX")).rejects.toBe(erreur);
  });
});
