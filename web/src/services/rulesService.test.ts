import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPatch: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { rulesService } from "./rulesService";

describe("rulesService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("getAll lit la collection sans filtre quand aucun serveur n'est passe", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await rulesService.getAll(null);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/rules");
  });

  it("getAll ajoute le filtre de serveur quand il est fourni", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await rulesService.getAll("g1");
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/rules?guild_id=g1",
    );
  });

  it("toggle met a jour la regle et renvoie l'etat demande", async () => {
    mocks.httpPatch.mockResolvedValue(undefined);

    await expect(rulesService.toggle("r1", false)).resolves.toBe(false);
    expect(mocks.httpPatch).toHaveBeenCalledWith("/api/rules/r1", {
      enabled: false,
    });
  });

  it("update envoie la regle complete quand les valeurs sont valides", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });
    const params = {
      guild_id: "g1",
      flag_type: "spam",
      weight: 5,
      threshold_warn: 3,
      threshold_delete: 6,
      threshold_mute: 80,
      threshold_ban: 90,
      enabled: true,
    };

    await rulesService.update(params);
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/rules", params);
  });

  it("update rejette un poids hors de l'intervalle [0, 10]", () => {
    const params = {
      guild_id: "g1",
      flag_type: "spam",
      weight: -1,
      threshold_warn: 3,
      threshold_delete: 6,
      threshold_mute: 80,
      threshold_ban: 90,
      enabled: true,
    };

    expect(() => rulesService.update(params)).toThrow(
      "Le poids doit etre entre 0 et 10",
    );
    expect(mocks.httpPost).not.toHaveBeenCalled();
  });

  it("update rejette un seuil hors de l'intervalle [0, 100]", () => {
    const params = {
      guild_id: "g1",
      flag_type: "spam",
      weight: 5,
      threshold_warn: -2,
      threshold_delete: 6,
      threshold_mute: 80,
      threshold_ban: 90,
      enabled: true,
    };

    expect(() => rulesService.update(params)).toThrow(
      "Le seuil warn doit etre entre 0 et 100",
    );
    expect(mocks.httpPost).not.toHaveBeenCalled();
  });
});
