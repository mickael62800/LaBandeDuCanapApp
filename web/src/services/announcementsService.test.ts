import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPatch: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { announcementsService } from "./announcementsService";

describe("announcementsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("list lit les annonces planifiees du serveur", async () => {
    const items = [{ id: "a1" }];
    mocks.httpGet.mockResolvedValue(items);

    await expect(announcementsService.list("g1")).resolves.toEqual(items);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/announcements/g1");
  });

  it("get lit une annonce par identifiant", async () => {
    const item = { id: "a1" };
    mocks.httpGet.mockResolvedValue(item);

    await expect(announcementsService.get("a1")).resolves.toEqual(item);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/announcements/by-id/a1");
  });

  it("create envoie le corps de la nouvelle annonce", async () => {
    const body = { guild_id: "g1", name: "Bienvenue", channel_ids: ["c1"] };
    mocks.httpPost.mockResolvedValue({ id: "a9", ...body });

    await expect(announcementsService.create(body)).resolves.toEqual({
      id: "a9",
      ...body,
    });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/announcements", body);
  });

  it("update envoie le patch de l'annonce par identifiant", async () => {
    const updated = { id: "a1", name: "Nouveau nom" };
    mocks.httpPatch.mockResolvedValue(updated);

    await expect(announcementsService.update("a1", { name: "Nouveau nom" }))
      .resolves.toEqual(updated);
    expect(mocks.httpPatch).toHaveBeenCalledWith("/api/announcements/by-id/a1", {
      name: "Nouveau nom",
    });
  });

  it("delete supprime l'annonce par identifiant", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await announcementsService.delete("a1");
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/announcements/by-id/a1",
    );
  });

  it("toggle active ou desactive l'annonce", async () => {
    mocks.httpPost.mockResolvedValue(true);

    await expect(announcementsService.toggle("a1", false)).resolves.toBe(true);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/announcements/a1/toggle", {
      enabled: false,
    });
  });

  it("preview renvoie l'annonce rendue telle qu'elle sera publiee", async () => {
    const rendered = { announcement_id: "a1" };
    mocks.httpGet.mockResolvedValue(rendered);

    await expect(announcementsService.preview("a1")).resolves.toEqual(
      rendered,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/announcements/a1/preview");
  });

  it("listRuns lit les executions avec la limite par defaut de 50", async () => {
    const runs = [{ id: "r1" }];
    mocks.httpGet.mockResolvedValue(runs);

    await expect(announcementsService.listRuns("a1")).resolves.toEqual(runs);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/announcements/a1/runs?limit=50");
  });

  it("listRuns accepte une limite explicite", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await announcementsService.listRuns("a1", 5);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/announcements/a1/runs?limit=5");
  });

  it("listButtonInteractions lit les clics sur boutons avec la limite par defaut de 100", async () => {
    const clicks = [{ id: "i1" }];
    mocks.httpGet.mockResolvedValue(clicks);

    await expect(announcementsService.listButtonInteractions("a1")).resolves.toEqual(clicks);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/announcements/a1/interactions?limit=100");
  });
});
