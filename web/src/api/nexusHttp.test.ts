import { beforeEach, describe, expect, it, vi } from "vitest";

// Tout est defini dans le bloc hoisted : il s'execute AVANT les imports du module sous test.
const mocks = vi.hoisted(() => {
  const request = vi.fn().mockResolvedValue({ ok: true });
  return { createBackendClient: vi.fn(() => request), request };
});

vi.mock("./backendHttp", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./backendHttp")>();
  return { ...actual, createBackendClient: (...args: unknown[]) => mocks.createBackendClient(...(args as [])) };
});

import { NexusHttpError, nexusDelete, nexusGet, nexusPatch, nexusPost, nexusPut } from "./nexusHttp";

describe("client Nexus (passerelle /nexus-api)", () => {
  beforeEach(() => {
    // NB: on ne fait PAS mockClear sur createBackendClient — le module cree son client a l'import.
    mocks.request.mockReset().mockResolvedValue({ ok: true });
  });

  it("configure le client sur /nexus-api avec les statuts vides Nexus", () => {
    expect(mocks.createBackendClient).toHaveBeenCalledTimes(1);
    const options = mocks.createBackendClient.mock.calls[0][0];
    expect(options.baseUrl).toBe("/nexus-api");
    expect(options.emptyStatuses).toEqual([202, 204]);
    expect(options.errorLabel).toBe("Nexus");
    expect(typeof options.makeError).toBe("function");
  });

  it("makeError produit bien une NexusHttpError", () => {
    const options = mocks.createBackendClient.mock.calls[0][0];
    const erreur = options.makeError("boom", { status: 502, body: null });

    expect(erreur).toBeInstanceOf(NexusHttpError);
    expect(erreur.name).toBe("NexusHttpError");
    expect(erreur.message).toBe("boom");
    expect(erreur.status).toBe(502);
  });

  it("nexusGet envoie GET + X-Guild-Id quand la guilde est fournie", async () => {
    await nexusGet("/api/x", "g1");
    expect(mocks.request).toHaveBeenCalledWith("GET", "/api/x", { headers: { "X-Guild-Id": "g1" } });

    // sans guilde : pas d'en-tete (undefined)
    mocks.request.mockClear();
    await nexusGet("/api/y", null);
    expect(mocks.request).toHaveBeenLastCalledWith("GET", "/api/y", { headers: undefined });
  });

  it.each([
    ["POST", "nexusPost"],
    ["PUT", "nexusPut"],
    ["PATCH", "nexusPatch"],
  ] as const)("%s envoie le corps et l'en-tete de guilde (%s)", async (method, label) => {
    const fn = { nexusPost, nexusPut, nexusPatch }[label];

    await fn("/api/z", "g9", { a: 1 });
    expect(mocks.request).toHaveBeenCalledWith(method, "/api/z", { body: { a: 1 }, headers: { "X-Guild-Id": "g9" } });

    // corps optionnel : undefined quand absent
    mocks.request.mockClear();
    await fn("/api/w", null);
    expect(mocks.request).toHaveBeenLastCalledWith(method, "/api/w", { body: undefined, headers: undefined });
  });

  it("nexusDelete envoie DELETE sans corps", async () => {
    await nexusDelete("/api/del/123", "g7");
    expect(mocks.request).toHaveBeenCalledWith("DELETE", "/api/del/123", { headers: { "X-Guild-Id": "g7" } });

    mocks.request.mockClear();
    await nexusDelete("/api/del/456", null);
    expect(mocks.request).toHaveBeenLastCalledWith("DELETE", "/api/del/456", { headers: undefined });
  });

  it("propage les erreurs du transport commun", async () => {
    const erreur = new Error("503");
    mocks.request.mockRejectedValue(erreur);
    await expect(nexusGet("/api/x", "g1")).rejects.toBe(erreur);
  });
});

describe("NexusHttpError (identite d'erreur propre a Nexus)", () => {
  it("est une instance de Error avec le nom attendu et les champs details", () => {
    const erreur = new NexusHttpError("Acces refuse.", { status: 403, body: null });

    expect(erreur).toBeInstanceOf(Error);
    expect(erreur.name).toBe("NexusHttpError");
    expect(erreur.message).toBe("Acces refuse.");
    expect(erreur.status).toBe(403);
    expect(erreur.body).toBeNull();
  });

  it("conserve le statut dans les champs individuels", () => {
    const erreur = new NexusHttpError("Session expiree — reconnecte-toi.", { status: 401, body: "nope" });
    expect(erreur.message).toBe("Session expiree — reconnecte-toi.");
    expect(erreur.status).toBe(401);
    expect(erreur.body).toBe("nope");
  });
});
