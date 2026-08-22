import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { aiDatasetService } from "./aiDatasetService";

describe("aiDatasetService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("listMessages sans parametres appelle la route nue du serveur", async () => {
    const response = { items: [], total: 0 };
    mocks.httpGet.mockResolvedValue(response);

    await expect(aiDatasetService.listMessages("g1")).resolves.toEqual(
      response,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/ai-dataset/messages/g1");
  });

  it("listMessages filtre les parametres vides et serialise le reste", async () => {
    mocks.httpGet.mockResolvedValue({ items: [], total: 0 });

    await aiDatasetService.listMessages("g1", {
      channel_id: "c9",
      from: "", // vide : ignore
      to: null, // null : ignore
      min_length: undefined, // undefined : ignore
      limit: 25,
      offset: 0, // zero est une valeur valide : conserve
    });

    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/ai-dataset/messages/g1?channel_id=c9&limit=25&offset=0",
    );
  });

  it("bulkDelete envoie les identifiants a supprimer dans le corps", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: 3 });

    await expect(
      aiDatasetService.bulkDelete("g1", ["m1", "m2", "m3"]),
    ).resolves.toEqual({ deleted: 3 });
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/ai-dataset/messages/g1",
      { ids: ["m1", "m2", "m3"] },
    );
  });
});
