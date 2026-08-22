import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
  httpPatch: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import {
  evidenceService,
  modstatsService,
  remindersService,
  reviewService,
} from "./moderationAdvancedService";

describe("remindersService (Phase 5)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("create publie un rappel de sanction", async () => {
    const cree = { id: "r1" };
    mocks.httpPost.mockResolvedValue(cree);
    const body = { guild_id: "g1", action_id: "a9", due_at: "2026-08-30T00:00:00Z" } as never;

    await expect(remindersService.create(body)).resolves.toBe(cree);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/reminders", body);
  });

  it("listByGuild lit les rappels de la guilde", async () => {
    const liste = [{ id: "r2" }];
    mocks.httpGet.mockResolvedValue(liste);

    await expect(remindersService.listByGuild("g5")).resolves.toBe(liste);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/reminders/g5");
  });

  it("getPending lit la file d'attente globale", async () => {
    const liste = [{ id: "r3" }];
    mocks.httpGet.mockResolvedValue(liste);

    await expect(remindersService.getPending()).resolves.toBe(liste);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/reminders/pending");
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("502");
    mocks.httpPost.mockRejectedValue(erreur);
    await expect(remindersService.create({} as never)).rejects.toBe(erreur);
  });
});

describe("evidenceService (Phase 5)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list lit les pieces jointes d'une action", async () => {
    const preuves = [{ id: "e1" }];
    mocks.httpGet.mockResolvedValue(preuves);

    await expect(evidenceService.list("a42")).resolves.toBe(preuves);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/moderation/evidence/a42");
  });

  it("add joint une piece a l'action", async () => {
    const cree = { id: "e9" };
    mocks.httpPost.mockResolvedValue(cree);
    const body = { action_id: "a1", kind: "screenshot", url: "https://x/y.png" } as never;

    await expect(evidenceService.add(body)).resolves.toBe(cree);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/moderation/evidence", body);
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("403");
    mocks.httpGet.mockRejectedValue(erreur);
    await expect(evidenceService.list("aX")).rejects.toBe(erreur);
  });
});

describe("reviewService (Phase 5)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("add soumet une action a relecture", async () => {
    const cree = { id: "v1" };
    mocks.httpPost.mockResolvedValue(cree);
    const body = { guild_id: "g1", action_id: "a7", requested_by: "m2" } as never;

    await expect(reviewService.add(body)).resolves.toBe(cree);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/moderation/review", body);
  });

  it("listPending lit la file de relecture d'une guilde", async () => {
    const liste = [{ id: "v2" }];
    mocks.httpGet.mockResolvedValue(liste);

    await expect(reviewService.listPending("g8")).resolves.toBe(liste);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/moderation/review/g8/pending",
    );
  });

  it("resolve tranche la relecture (PATCH + corps)", async () => {
    const body = { decision: "uphold", decided_by: "m9" } as never;
    await reviewService.resolve("v3", body);

    expect(mocks.httpPatch).toHaveBeenCalledWith(
      "/api/moderation/review/v3/resolve",
      body,
    );
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("409");
    mocks.httpPatch.mockRejectedValue(erreur);
    await expect(reviewService.resolve("vX", {} as never)).rejects.toBe(erreur);
  });
});

describe("modstatsService (Phase 5)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list utilise la fenetre par defaut de 30 jours", async () => {
    const stats = [{ day: "2026-08-21" }];
    mocks.httpGet.mockResolvedValue(stats);

    await expect(modstatsService.list("g1")).resolves.toBe(stats);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/moderation/modstats/g1?days=30",
    );
  });

  it("list honore une fenetre explicite", async () => {
    await modstatsService.list("g2", 7);
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/moderation/modstats/g2?days=7",
    );
  });

  it("trend lit la courbe journaliere (defaut puis explicite)", async () => {
    const tendance = [{ day: "d1", warns: 0, mutes: 1, bans: 0, kicks: 2 }];
    mocks.httpGet.mockResolvedValue(tendance);

    await expect(modstatsService.trend("g3")).resolves.toBe(tendance);
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/moderation/modstats/g3/trend?days=30",
    );

    await modstatsService.trend("g4", 14);
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/moderation/modstats/g4/trend?days=14",
    );
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("500");
    mocks.httpGet.mockRejectedValue(erreur);
    await expect(modstatsService.list("gX")).rejects.toBe(erreur);
  });
});
