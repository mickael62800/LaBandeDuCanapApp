import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  atriumDelete: vi.fn(),
  atriumGet: vi.fn(),
  atriumPut: vi.fn(),
}));

vi.mock("@/api/atriumHttp", () => mocks);

import { atriumService } from "./atriumService";

describe("atriumService (admin Atrium)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("state lit l'etat d'Atrium pour le serveur", async () => {
    const state = { guild_id: "g1", enabled: true };
    mocks.atriumGet.mockResolvedValue(state);

    await expect(atriumService.state("g1")).resolves.toEqual(state);
    expect(mocks.atriumGet).toHaveBeenCalledWith("/admin/guilds/g1/state");
  });

  it("setState ecrit l'etat avec l'acteur a la cle", async () => {
    const state = { guild_id: "g1", enabled: false };
    mocks.atriumPut.mockResolvedValue(state);

    await expect(atriumService.setState("g1", false, "a9")).resolves.toEqual(
      state,
    );
    expect(mocks.atriumPut).toHaveBeenCalledWith("/admin/guilds/g1/state", {
      enabled: false,
      actor_id: "a9",
    });
  });

  it("usage lit la consommation et les limites reelles du serveur", async () => {
    const usage = { global_used_today: 3 };
    mocks.atriumGet.mockResolvedValue(usage);

    await expect(atriumService.usage("g1")).resolves.toEqual(usage);
    expect(mocks.atriumGet).toHaveBeenCalledWith("/admin/guilds/g1/usage");
  });

  it("context lit les consignes de ton enregistrees", async () => {
    const context = { welcome_context: "chaleureux" };
    mocks.atriumGet.mockResolvedValue(context);

    await expect(atriumService.context("g1")).resolves.toEqual(context);
    expect(mocks.atriumGet).toHaveBeenCalledWith("/admin/guilds/g1/config");
  });

  it("setConfig n'ecrit que les cles passees", async () => {
    const result = { updated: 2 };
    mocks.atriumPut.mockResolvedValue(result);

    await expect(
      atriumService.setConfig("g1", { welcome_context: "calme" }),
    ).resolves.toEqual({ updated: 2 });
    expect(mocks.atriumPut).toHaveBeenCalledWith("/admin/guilds/g1/config", {
      values: { welcome_context: "calme" },
    });
  });

  it("knowledge liste les documents de la base du serveur", async () => {
    const docs = [{ id: "d1", title: "Règles" }];
    mocks.atriumGet.mockResolvedValue(docs);

    await expect(atriumService.knowledge("g1")).resolves.toEqual(docs);
    expect(mocks.atriumGet).toHaveBeenCalledWith("/admin/guilds/g1/knowledge");
  });

  it("forgetMember efface la memoire d'un membre et trace l'acteur", async () => {
    const result = { guild_id: "g1", member_id: "m2", deleted: 7 };
    mocks.atriumDelete.mockResolvedValue(result);

    await expect(
      atriumService.forgetMember("g1", "m2", "a9"),
    ).resolves.toEqual(result);
    expect(mocks.atriumDelete).toHaveBeenCalledWith(
      "/admin/guilds/g1/members/m2/memory",
      { actor_id: "a9" },
    );
  });
});
