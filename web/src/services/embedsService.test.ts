import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
  httpPut: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { embedsService } from "./embedsService";

const entree = { id: "e1", guild_id: "g1" };
const corps = { name: "Bienvenue", fields: [] };

describe("embedsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("list lit les embeds d'un serveur", async () => {
    mocks.httpGet.mockResolvedValue([entree]);

    await expect(embedsService.list("g1")).resolves.toEqual([entree]);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/embeds/g1");
  });

  it("get lit un embed par son id", async () => {
    mocks.httpGet.mockResolvedValue(entree);

    await expect(embedsService.get("e1")).resolves.toEqual(entree);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/embeds/by-id/e1");
  });

  it("create enregistre un embed dans le serveur", async () => {
    mocks.httpPost.mockResolvedValue(entree);

    await expect(embedsService.create("g1", corps)).resolves.toEqual(entree);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/embeds/g1", corps);
  });

  it("update modifie un embed existant", async () => {
    mocks.httpPut.mockResolvedValue(entree);

    await expect(embedsService.update("e1", corps)).resolves.toEqual(entree);
    expect(mocks.httpPut).toHaveBeenCalledWith("/api/embeds/by-id/e1", corps);
  });

  it("remove supprime un embed", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await embedsService.remove("e1");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/embeds/by-id/e1");
  });

  it("post publie l'embed dans le canal choisi", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await expect(embedsService.post("e1", "c9")).resolves.toEqual({ ok: true });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/embeds/by-id/e1/post", {
      channel_id: "c9",
    });
  });

  it("editPosted relance la publication de l'embed deja envoye", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await expect(embedsService.editPosted("e1")).resolves.toEqual({ ok: true });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/embeds/by-id/e1/edit", {});
  });
});
