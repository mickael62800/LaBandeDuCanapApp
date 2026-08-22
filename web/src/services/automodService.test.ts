import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { automodService } from "./automodService";

describe("automodService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("fpStats lit le taux de faux positifs sur la periode par defaut", async () => {
    const stats = { days: 30, overall: {} };
    mocks.httpGet.mockResolvedValue(stats);

    await expect(automodService.fpStats("g1")).resolves.toEqual(stats);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/automod/g1/fp-stats?days=30");
  });

  it("fpStats accepte une periode explicite", async () => {
    mocks.httpGet.mockResolvedValue({ days: 7, overall: {} });

    await automodService.fpStats("g1", 7);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/automod/g1/fp-stats?days=7");
  });

  it("listDetections sans filtre lit toute la timeline du serveur", async () => {
    const items = [{ id: "d1" }];
    mocks.httpGet.mockResolvedValue(items);

    await expect(automodService.listDetections("g1")).resolves.toEqual(items);
    // Aucun parametre : pas de query string.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/automod/g1/detections");
  });

  it("listDetections filtre par utilisateur et pagination", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await automodService.listDetections("g1", { user_id: "u2", limit: 5, offset: 10 });
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/automod/g1/detections?user_id=u2&limit=5&offset=10",
    );
  });

  it("listReviews lit les cartes pending par defaut (sans query)", async () => {
    const reviews = [{ id: "r1" }];
    mocks.httpGet.mockResolvedValue(reviews);

    await expect(automodService.listReviews("g1")).resolves.toEqual(reviews);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/automod/g1/reviews");
  });

  it("listReviews inclut les resolues avec une limite", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await automodService.listReviews("g1", { include_resolved: true, limit: 20 });
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/automod/g1/reviews?include_resolved=true&limit=20",
    );
  });

  it("getDiscussionMessages lit le transcript du salon de discussion", async () => {
    const messages = [{ discord_message_id: "dm1" }];
    mocks.httpGet.mockResolvedValue(messages);

    await expect(automodService.getDiscussionMessages("r1")).resolves.toEqual(
      messages,
    );
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/automod/reviews/r1/discussion/messages");
  });

  it("resolveReview applique la decision avec l'identite du decideur", async () => {
    const review = { id: "r1", status: "applied" };
    mocks.httpPost.mockResolvedValue(review);

    await expect(
      automodService.resolveReview("r1", {
        applied_action: "warn",
        resolved_by_id: "a9",
        resolved_by_name: "micka",
      }),
    ).resolves.toEqual({ id: "r1", status: "applied" });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/automod/reviews/r1/resolve", {
      applied_action: "warn",
      resolved_by_id: "a9",
      resolved_by_name: "micka",
    });
  });
});
