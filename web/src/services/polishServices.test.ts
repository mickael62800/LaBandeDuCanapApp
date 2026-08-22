import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { sponsorshipsService, systemOpsService, tempRolesService } from "./polishServices";

describe("sponsorshipsService (Phase 10)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list lit les sponsorships de la guilde", async () => {
    const liste = [{ id: "s1" }];
    mocks.httpGet.mockResolvedValue(liste);

    await expect(sponsorshipsService.list("g1")).resolves.toBe(liste);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/sponsorships/g1");
  });

  it("create envoie le payload complet", async () => {
    const corps = { sponsor_id: "u9", amount_cents: 500, months: 3 };
    mocks.httpPost.mockResolvedValue({ id: "s2" });

    await expect(sponsorshipsService.create(corps)).resolves.toEqual({ id: "s2" });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/sponsorships", corps);
  });
});

describe("tempRolesService (Phase 10)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("list lit les temp-roles de la guilde", async () => {
    const liste = [{ id: "t1" }];
    mocks.httpGet.mockResolvedValue(liste);

    await expect(tempRolesService.list("g2")).resolves.toBe(liste);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/temp-roles/g2");
  });

  it("create envoie le payload", async () => {
    const corps = { user_id: "u1", role_id: "r1", expires_at: "2026-09-01T00:00:00.000Z" };
    mocks.httpPost.mockResolvedValue({ id: "t2" });

    await expect(tempRolesService.create(corps)).resolves.toEqual({ id: "t2" });
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/temp-roles", corps);
  });

  it("remove supprime le temp-role du couple utilisateur/role", async () => {
    mocks.httpDelete.mockResolvedValue(undefined);

    await tempRolesService.remove("g3", "u4", "r5");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/temp-roles/g3/u4/r5");
  });
});

describe("systemOpsService (Phase 10)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("getModelsStatus lit l'etat des modeles", async () => {
    const statut = { models: {} };
    mocks.httpGet.mockResolvedValue(statut);

    await expect(systemOpsService.getModelsStatus()).resolves.toBe(statut);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/models/status");
  });

  it("reloadModel demande le rechargement d'un type de modele", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await systemOpsService.reloadModel("vision");
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/models/reload", { model_type: "vision" });
  });

  it("getCacheStats lit les statistiques de cache", async () => {
    const stats = { hits: 10, misses: 2 };
    mocks.httpGet.mockResolvedValue(stats);

    await expect(systemOpsService.getCacheStats()).resolves.toBe(stats);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/cache/stats");
  });
});
