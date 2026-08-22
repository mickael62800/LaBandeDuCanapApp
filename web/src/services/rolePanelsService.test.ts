import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
  httpDelete: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { rolePanelsService } from "./rolePanelsService";

describe("rolePanelsService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("getAll lit les panneaux de la guilde", async () => {
    const panneaux = [{ id: "p1" }];
    mocks.httpGet.mockResolvedValue(panneaux);

    await expect(rolePanelsService.getAll("g1")).resolves.toBe(panneaux);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/role-panels/g1");
  });

  it("getDetail lit le panneau avec ses entrees", async () => {
    const detail = { id: "p2", entries: [] };
    mocks.httpGet.mockResolvedValue(detail);

    await expect(rolePanelsService.getDetail("p2")).resolves.toBe(detail);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/role-panels/detail/p2");
  });

  it("create publie un panneau complet (mode, max_roles, entrees)", async () => {
    const cree = { id: "p3" };
    mocks.httpPost.mockResolvedValue(cree);
    const body = {
      guild_id: "g9",
      channel_id: "c1",
      title: "Choisis ton role",
      mode: "button",
      max_roles: 2,
      entries: [
        { role_id: "r1", role_name: "Anime", emoji: "\u{1F3E6}", style: "primary" },
        { role_id: "r2", role_name: "Sport" },
      ],
    };

    await expect(rolePanelsService.create(body)).resolves.toBe(cree);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/role-panels", body);
  });

  it("remove supprime le panneau cible", async () => {
    await rolePanelsService.remove("p4");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/role-panels/detail/p4");
  });

  it("getAutoRoles lit les roles automatiques de la guilde", async () => {
    const configs = [{ id: "ar1" }];
    mocks.httpGet.mockResolvedValue(configs);

    await expect(rolePanelsService.getAutoRoles("g2")).resolves.toBe(configs);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/auto-roles/g2");
  });

  it("addAutoRole enregistre un role automatique (delai optionnel)", async () => {
    const cree = { id: "ar9" };
    mocks.httpPost.mockResolvedValue(cree);
    const body = { guild_id: "g3", role_id: "r7", role_name: "VIP", delay_secs: 60 };

    await expect(rolePanelsService.addAutoRole(body)).resolves.toBe(cree);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/auto-roles", body);
  });

  it("removeAutoRole retire le couple guilde/role", async () => {
    await rolePanelsService.removeAutoRole("g4", "r8");
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/auto-roles/g4/r8");
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("501");
    mocks.httpPost.mockRejectedValue(erreur);
    await expect(rolePanelsService.create({} as never)).rejects.toBe(erreur);
  });
});
