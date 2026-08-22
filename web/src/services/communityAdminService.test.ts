import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
  httpPut: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { communityAdminService } from "./communityAdminService";

describe("communityAdminService (back-office communautaire)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("listLfg lit toutes les annonces par defaut (?all=1)", async () => {
    const items = [{ id: "l1" }];
    mocks.httpGet.mockResolvedValue(items);

    await expect(communityAdminService.listLfg("g1")).resolves.toEqual(items);
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/lfg/g1?all=1");
  });

  it("listLfg sans all ne demande que les annonces ouvertes", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await communityAdminService.listLfg("g/1", false);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/lfg/g%2F1");
  });

  it("closeLfg ferme l'annonce sans la supprimer", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await expect(communityAdminService.closeLfg("l1")).resolves.toEqual({
      ok: true,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/lfg/detail/l1/close", {});
  });

  it("deleteLfg supprime definitivement l'annonce", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: true });

    await expect(communityAdminService.deleteLfg("l1")).resolves.toEqual({
      deleted: true,
    });
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/lfg/detail/l1");
  });

  it("listPolls lit tous les sondages par defaut", async () => {
    const polls = [{ id: "p1" }];
    mocks.httpGet.mockResolvedValue(polls);

    await expect(communityAdminService.listPolls("g1")).resolves.toEqual(polls);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/polls/g1?all=1");
  });

  it("createPoll publie un nouveau sondage", async () => {
    const input = { question: "Quand ?", closes_at: "2026-09-01T00:00:00Z" };
    mocks.httpPost.mockResolvedValue({ id: "p9", ...input });

    await expect(communityAdminService.createPoll("g1", input)).resolves.toEqual({
      id: "p9",
      ...input,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/polls/g1", input);
  });

  it("closePoll clot le sondage", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await communityAdminService.closePoll("p1");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/polls/detail/p1/close", {});
  });

  it("deletePoll supprime le sondage", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: true });

    await communityAdminService.deletePoll("p1");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/polls/detail/p1");
  });

  it("listSpotlight lit les membres du mois designes", async () => {
    const items = [{ id: "s1" }];
    mocks.httpGet.mockResolvedValue(items);

    await expect(communityAdminService.listSpotlight("g1")).resolves.toEqual(items);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/spotlight/g1");
  });

  it("designate remplace le membre du mois de la periode", async () => {
    const input = { user_id: "u2", reason: "toujours la" };
    mocks.httpPost.mockResolvedValue({ id: "s9", ...input });

    await expect(communityAdminService.designate("g1", input)).resolves.toEqual({
      id: "s9",
      ...input,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/spotlight/g1", input);
  });

  it("deleteSpotlight retire la designation d'un membre du mois", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: true });

    await communityAdminService.deleteSpotlight("g1", "s1");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/spotlight/g1/detail/s1");
  });

  it("listNews lit toutes les nouvelles par defaut", async () => {
    const items = [{ id: "n1" }];
    mocks.httpGet.mockResolvedValue(items);

    await expect(communityAdminService.listNews("g1")).resolves.toEqual(items);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/news/g1?all=1");
  });

  it("createNews publie une nouvelle", async () => {
    const input = { title: "T", body: "B" };
    mocks.httpPost.mockResolvedValue({ id: "n9", ...input });

    await expect(communityAdminService.createNews("g1", input)).resolves.toEqual({
      id: "n9",
      ...input,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/news/g1", input);
  });

  it("updateNews modifie une nouvelle existante", async () => {
    const input = { title: "T2" };
    mocks.httpPut.mockResolvedValue({ id: "n1", ...input });

    await expect(communityAdminService.updateNews("n1", input)).resolves.toEqual({
      id: "n1",
      ...input,
    });
    expect(mocks.httpPut).toHaveBeenCalledWith("/api/news/detail/n1", input);
  });

  it("deleteNews supprime une nouvelle", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: true });

    await communityAdminService.deleteNews("n1");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/news/detail/n1");
  });

  it("listEvents sans dates lit tous les evenements du serveur", async () => {
    const items = [{ id: "e1" }];
    mocks.httpGet.mockResolvedValue(items);

    await communityAdminService.listEvents("g1").then((r) => expect(r).toEqual(items));
    // Pas de dates : pas de query string.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/events/g1");
  });

  it("listEvents avec bornes serialise la plage ISO", async () => {
    mocks.httpGet.mockResolvedValue([]);

    const from = new Date(Date.UTC(2026, 7, 1));
    const to = new Date(Date.UTC(2026, 8, 30));
    await communityAdminService.listEvents("g1", from, to);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      `/api/events/g1?from=${encodeURIComponent(from.toISOString())}&to=${encodeURIComponent(to.toISOString())}`,
    );
  });

  it("createEvent publie un evenement", async () => {
    const input = { title: "Tournoi", starts_at: "2026-09-01T20:00:00Z" };
    mocks.httpPost.mockResolvedValue({ id: "e9" });

    await expect(communityAdminService.createEvent("g1", input)).resolves.toEqual({
      id: "e9",
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/events/g1", input);
  });

  it("updateEvent modifie un evenement existant", async () => {
    const input = { title: "Tournoi v2" };
    mocks.httpPut.mockResolvedValue({ id: "e1" });

    await communityAdminService.updateEvent("e1", input).then((r) => expect(r).toEqual({ id: "e1" }));
    expect(mocks.httpPut).toHaveBeenCalledWith("/api/events/detail/e1", input);
  });

  it("deleteEvent supprime un evenement", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: true });

    await communityAdminService.deleteEvent("e1");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/events/detail/e1");
  });
});
