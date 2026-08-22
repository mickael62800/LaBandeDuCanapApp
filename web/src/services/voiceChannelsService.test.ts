import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
  httpPatch: vi.fn(),
  httpDelete: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { voiceChannelsService, voiceThemesService } from "./voiceChannelsService";

describe("voiceThemesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list lit les themes de la guilde", async () => {
    const themes = [{ id: "t1" }];
    mocks.httpGet.mockResolvedValue(themes);

    await expect(voiceThemesService.list("g1")).resolves.toBe(themes);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/voice-channels/themes/g1");
  });

  it("create publie un theme", async () => {
    const cree = { id: "t2" };
    mocks.httpPost.mockResolvedValue(cree);
    const body = { name: "Neon", color: "#ff0044" } as never;

    await expect(voiceThemesService.create("g1", body)).resolves.toBe(cree);
    expect(mocks.httpPost).toHaveBeenCalledWith(
      "/api/voice-channels/themes/g1",
      body,
    );
  });

  it("update patche le theme cible", async () => {
    const maj = { id: "t2" };
    mocks.httpPatch.mockResolvedValue(maj);

    await expect(
      voiceThemesService.update("g1", "t2", { name: "Retro" } as never),
    ).resolves.toBe(maj);
    expect(mocks.httpPatch).toHaveBeenCalledWith(
      "/api/voice-channels/themes/g1/t2",
      { name: "Retro" },
    );
  });

  it("remove supprime le theme cible", async () => {
    await voiceThemesService.remove("g1", "t3");
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/voice-channels/themes/g1/t3",
    );
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("404");
    mocks.httpGet.mockRejectedValue(erreur);
    await expect(voiceThemesService.list("gX")).rejects.toBe(erreur);
  });
});

describe("voiceChannelsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("getAll sans guilde interroge la vue globale _all", async () => {
    await voiceChannelsService.getAll();
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/voice-channels/_all");

    await voiceChannelsService.getAll(null);
    expect(mocks.httpGet).toHaveBeenLastCalledWith("/api/voice-channels/_all");
  });

  it("getAll avec guilde interroge la vue de cette guilde", async () => {
    const salons = [{ id: "c1" }];
    mocks.httpGet.mockResolvedValue(salons);

    await expect(voiceChannelsService.getAll("g7")).resolves.toBe(salons);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/voice-channels/g7");
  });

  it("getHistory sans limite garde la route nue", async () => {
    await voiceChannelsService.getHistory("g1");
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/voice-channels/g1/history",
    );
  });

  it("getHistory avec limite ajoute le plafond encode", async () => {
    await voiceChannelsService.getHistory("g1", 50);
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/voice-channels/g1/history?limit=50",
    );
  });

  it("getDetail lit le salon par identifiant de canal", async () => {
    const detail = { id: "c9" };
    mocks.httpGet.mockResolvedValue(detail);

    await expect(voiceChannelsService.getDetail("c9")).resolves.toBe(detail);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/voice-channels/by-channel/c9");
  });

  it("close ferme le salon (soft-delete)", async () => {
    await voiceChannelsService.close("c10");
    expect(mocks.httpPatch).toHaveBeenCalledWith(
      "/api/voice-channels/by-channel/c10/close",
    );
  });

  it("purge efface les lignes du salon", async () => {
    await voiceChannelsService.purge("c11");
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/voice-channels/by-channel/c11/purge",
    );
  });

  it("purgeHistory vide l'historique de la guilde", async () => {
    const resultat = { deleted: 42 };
    mocks.httpDelete.mockResolvedValue(resultat);

    await expect(voiceChannelsService.purgeHistory("g3")).resolves.toBe(
      resultat,
    );
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/voice-channels/g3/history");
  });

  it("getEvents utilise le plafond par defaut puis explicite", async () => {
    await voiceChannelsService.getEvents("c12");
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/voice-channels/by-channel/c12/events?limit=200",
    );

    await voiceChannelsService.getEvents("c12", 5);
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/voice-channels/by-channel/c12/events?limit=5",
    );
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("503");
    mocks.httpDelete.mockRejectedValue(erreur);
    await expect(voiceChannelsService.purgeHistory("g1")).rejects.toBe(erreur);
  });
});
