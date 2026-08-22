import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpGet: vi.fn(),
  httpDelete: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

import { securityService } from "./securityService";

describe("securityService", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset().mockResolvedValue([]);
  });

  it("getEvents sans guilde interroge la route nue", async () => {
    await securityService.getEvents();
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/security/events");

    await securityService.getEvents(null);
    expect(mocks.httpGet).toHaveBeenLastCalledWith("/api/security/events");
  });

  it("getEvents avec guilde ajoute le filtre encode", async () => {
    await securityService.getEvents("guild-123");
    expect(mocks.httpGet).toHaveBeenCalledWith(
      "/api/security/events?guild_id=guild-123",
    );

    // Les caracteres speciaux sont encodes par l'helper q().
    await securityService.getEvents("g 1&x=2");
    expect(mocks.httpGet).toHaveBeenLastCalledWith(
      "/api/security/events?guild_id=g%201%26x%3D2",
    );
  });

  it("purge supprime les events de la guilde ciblee", async () => {
    const resultat = { deleted_events: 4, deleted_watches: 2 };
    mocks.httpDelete.mockResolvedValue(resultat);

    await expect(securityService.purge("guild-1")).resolves.toBe(resultat);
    expect(mocks.httpDelete).toHaveBeenCalledWith(
      "/api/security/events/guild-1",
    );
  });

  it("propage les erreurs du client HTTP", async () => {
    const erreur = new Error("403");
    mocks.httpGet.mockRejectedValue(erreur);
    await expect(securityService.getEvents()).rejects.toBe(erreur);
  });
});
