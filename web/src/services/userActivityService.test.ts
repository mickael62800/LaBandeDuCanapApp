import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ httpGet: vi.fn() }));

vi.mock("@/api/http", () => mocks);

import { userActivityService } from "./userActivityService";

describe("userActivityService (activite d'un utilisateur)", () => {
  beforeEach(() => {
    mocks.httpGet.mockReset().mockResolvedValue([]);
  });

  it("list lit l'activite du couple guilde/utilisateur sans options", async () => {
    const activites = [{ id: "a1", event_type: "message" }];
    mocks.httpGet.mockResolvedValue(activites);

    await expect(userActivityService.list("g1", "u2")).resolves.toBe(activites);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/user-activity/g1/u2");
  });

  it("list serialise les options fournies et saute celles absentes", async () => {
    await userActivityService.list("g3", "u4", { eventType: "join", limit: 5, offset: 0 });

    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/user-activity/g3/u4?event_type=join&limit=5&offset=0",
    );

    await userActivityService.list("g5", "u6", { limit: 10 });
    expect(mocks.httpGet).toHaveBeenLastCalledWith("/api/user-activity/g5/u6?limit=10");
  });

  it("propage les erreurs du transport", async () => {
    const erreur = new Error("404");
    mocks.httpGet.mockRejectedValue(erreur);
    await expect(userActivityService.list("gX", "uY")).rejects.toBe(erreur);
  });
});
