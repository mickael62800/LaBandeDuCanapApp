import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { systemService } from "./systemService";

describe("systemService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue({});
  });

  it("getInfo interroge l'etat complet de la plateforme", async () => {
    const info = { bots: [], workers: [], uptime_seconds: 10 };
    mocks.httpGet.mockResolvedValue(info);

    await expect(systemService.getInfo()).resolves.toBe(info);
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/system/info");
  });

  it("resetGuild envoie la confirmation et les options par defaut (tout actif)", async () => {
    const resultat = { tables_wiped: 12, total_rows: 340 };
    mocks.httpPost.mockResolvedValue(resultat);

    await expect(systemService.resetGuild("g1", "Ma Bande")).resolves.toBe(
      resultat,
    );
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/system/guild-reset/g1", {
      confirmation: "Ma Bande",
      unban: true,
      unmute: true,
      remove_roles: true,
    });
  });

  it("resetGuild honore les options explicites (y compris false)", async () => {
    await systemService.resetGuild("g2", "Bande", {
      unban: false,
      unmute: true,
      remove_roles: undefined, // non fournie : retombe sur le defaut true
    });

    expect(mocks.httpPost).toHaveBeenLastCalledWith(
      "/api/system/guild-reset/g2",
      { confirmation: "Bande", unban: false, unmute: true, remove_roles: true },
    );
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("403");
    mocks.httpPost.mockRejectedValue(erreur);
    await expect(systemService.resetGuild("g1", "x")).rejects.toBe(erreur);
  });
});
