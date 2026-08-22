import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { confessionsService } from "./confessionsService";

describe("confessionsService (moderation des confessions)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("list lit les confessions du serveur avec la limite par defaut", async () => {
    const items = [{ id: "c1" }];
    mocks.httpGet.mockResolvedValue(items);

    await expect(confessionsService.list("g1")).resolves.toEqual(items);
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/confessions/g1/list?limit=100&include_deleted=false",
    );
  });

  it("list avec includeDeleted lit aussi les supprimees", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await confessionsService.list("g/2", true, 5).then((r) => expect(r).toEqual([]));
    // `guildId` est interpole tel quel (pas d'encodeURIComponent ici).
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/confessions/g/2/list?limit=5&include_deleted=true",
    );
  });

  it("delete supprime une confession avec auteur et motif optionnel", async () => {
    mocks.httpDelete.mockResolvedValue({ id: "c1" });

    await expect(confessionsService.delete("c1", "mod-9")).resolves.toEqual({ id: "c1" });
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/confessions/by-id/c1", {
      deleted_by: "mod-9",
      reason: undefined,
    });

    mocks.httpDelete.mockClear();
    await confessionsService.delete("c2", "mod-9", "spam").then((r) => expect(r).toEqual({ id: "c1" }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/confessions/by-id/c2", {
      deleted_by: "mod-9",
      reason: "spam",
    });
  });

  it("listReplies lit les reponses d'une confession", async () => {
    const replies = [{ id: "r1" }];
    mocks.httpGet.mockResolvedValue(replies);

    await expect(confessionsService.listReplies("c1")).resolves.toEqual(replies);
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/confessions/by-id/c1/replies");
  });

  it("deleteReply supprime une reponse", async () => {
    mocks.httpDelete.mockResolvedValue({ id: "r1" });

    await confessionsService.deleteReply("r1", "mod-9").then((r) => expect(r).toEqual({ id: "r1" }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/confessions/replies/r1", {
      deleted_by: "mod-9",
    });
  });

  it("listReports lit les signalements avec statut optionnel et limite par defaut", async () => {
    const reports = [{ id: "rep1" }];
    mocks.httpGet.mockResolvedValue(reports);

    await expect(confessionsService.listReports("g1")).resolves.toEqual(reports);
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/confessions/g1/reports?limit=50");

    mocks.httpGet.mockClear();
    await confessionsService
      .listReports("g/2", "pending", 7)
      .then((r) => expect(r).toEqual(reports));
    // `guildId` est interpole tel quel (pas d'encodeURIComponent ici).
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/confessions/g/2/reports?status=pending&limit=7");
  });

  it("resolveReport clot ou classe sans suite un signalement", async () => {
    mocks.httpPost.mockResolvedValue(undefined);

    await confessionsService.resolveReport("rep1", "resolved", "mod-9").then((r) => expect(r).toBeUndefined());
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/confessions/reports/rep1/resolve", {
      status: "resolved",
      resolved_by: "mod-9",
    });

    mocks.httpPost.mockClear();
    await confessionsService.resolveReport("rep2", "dismissed", "mod-9").then((r) => expect(r).toBeUndefined());
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/confessions/reports/rep2/resolve", {
      status: "dismissed",
      resolved_by: "mod-9",
    });
  });
});
