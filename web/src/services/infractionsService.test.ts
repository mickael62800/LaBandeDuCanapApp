import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ httpDelete: vi.fn(), httpGet: vi.fn() }));

vi.mock("@/api/http", () => mocks);

import { infractionsService } from "./infractionsService";

describe("infractionsService (journal des infractions)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue([]);
  });

  it("getAll lit le journal sans filtre de guilde", async () => {
    const liste = [{ id: "i1" }];
    mocks.httpGet.mockResolvedValue(liste);

    await expect(infractionsService.getAll()).resolves.toBe(liste);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/infractions");
  });

  it("getAll filtre par guilde quand elle est fournie", async () => {
    await infractionsService.getAll("g1");
    expect(mocks.httpGet).toHaveBeenLastCalledWith("/api/infractions?guild_id=g1");
  });

  it("remove sans source cible la table automod /api/infractions/{id}", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await infractionsService.remove("i9");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/infractions/i9");
  });

  it("remove avec source action cible la moderation (unban eventuel)", async () => {
    mocks.httpDelete.mockReset().mockResolvedValue(undefined);

    await infractionsService.remove("a7", "action");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/moderation/actions/a7");
  });

  it("purgeAll vide tout le journal de la guilde (days: 0)", async () => {
    const resultat = { deleted: 12, points_restored: 3 };
    mocks.httpDelete.mockResolvedValue(resultat);

    await expect(infractionsService.purgeAll("g5")).resolves.toBe(resultat);
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/purge/infractions", {
      guild_id: "g5",
      days: 0,
    });
  });

  it("propage les erreurs du transport", async () => {
    const erreur = new Error("403");
    mocks.httpGet.mockRejectedValue(erreur);
    await expect(infractionsService.getAll()).rejects.toBe(erreur);
  });
});
