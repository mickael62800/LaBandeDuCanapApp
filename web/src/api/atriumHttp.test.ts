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

import { AtriumHttpError, atriumDelete, atriumGet, atriumPut } from "./atriumHttp";

describe("client Atrium (passerelle /atrium-api)", () => {
  beforeEach(() => {
    // NB: on ne fait PAS mockClear sur createBackendClient — le module cree son client a l'import.
    mocks.request.mockReset().mockResolvedValue({ ok: true });
  });

  it("configure le client sur /atrium-api avec retry 502/503 et message d'unavailable", () => {
    expect(mocks.createBackendClient).toHaveBeenCalledTimes(1);
    const options = mocks.createBackendClient.mock.calls[0][0];
    expect(options.baseUrl).toBe("/atrium-api");
    expect(options.errorLabel).toBe("Atrium");
    expect(options.forbiddenMessage).toBe("Accès à Atrium refusé.");
    expect(options.unavailableMessage).toContain("Atrium ne répond pas");
    // 404 definitif : jamais rejoue. Ecritures non rejouees (double effet).
    expect(options.retryStatuses).toEqual([502, 503]);
    expect(typeof options.makeError).toBe("function");
  });

  it("atriumGet envoie GET sans corps", async () => {
    await atriumGet("/api/x");
    expect(mocks.request).toHaveBeenCalledWith("GET", "/api/x");
  });

  it.each([["PUT", "atriumPut"] as const])(
    "%s envoie le corps quand fourni, sinon rien (%s)",
    async (method, label) => {
      const fn = { atriumPut }[label];

      await fn("/api/z", { a: 1 });
      expect(mocks.request).toHaveBeenCalledWith(method, "/api/z", { body: { a: 1 } });

      // corps optionnel : undefined quand absent (le transport ne stringifie pas)
      mocks.request.mockClear();
      await fn("/api/w");
      expect(mocks.request).toHaveBeenLastCalledWith(method, "/api/w", { body: undefined });
    },
  );

  it("atriumDelete envoie DELETE avec le corps (actor_id trace l'effacement)", async () => {
    await atriumDelete("/api/del/123", { actor_id: "u9" });
    expect(mocks.request).toHaveBeenCalledWith("DELETE", "/api/del/123", { body: { actor_id: "u9" } });

    mocks.request.mockClear();
    await atriumDelete("/api/del/456");
    expect(mocks.request).toHaveBeenLastCalledWith("DELETE", "/api/del/456", { body: undefined });
  });

  it("propage les erreurs du transport commun", async () => {
    const erreur = new AtriumHttpError("Atrium ne répond pas.", { status: 503, body: null });
    mocks.request.mockRejectedValue(erreur);
    await expect(atriumGet("/api/x")).rejects.toBe(erreur);
  });

  it("makeError produit bien une AtriumHttpError", () => {
    const options = mocks.createBackendClient.mock.calls[0][0];
    const erreur = options.makeError("boom", { status: 502, body: null });
    expect(erreur).toBeInstanceOf(AtriumHttpError);
    expect((erreur as AtriumHttpError).name).toBe("AtriumHttpError");
  });
});

describe("AtriumHttpError (identite d'erreur propre a Atrium)", () => {
  it("est une instance de BackendHttpError avec le nom attendu et les champs details", async () => {
    const actual = await import("./backendHttp");
    const erreur = new AtriumHttpError("Acces refuse.", { status: 403, body: null });

    expect(erreur).toBeInstanceOf(Error);
    expect(erreur).toBeInstanceOf(actual.BackendHttpError);
    expect(erreur.name).toBe("AtriumHttpError");
    expect(erreur.message).toBe("Acces refuse.");
    expect(erreur.status).toBe(403);
    expect(erreur.body).toBeNull();
  });

  it("conserve le statut dans les champs individuels", () => {
    const erreur = new AtriumHttpError("Session expiree — reconnecte-toi.", { status: 401, body: "nope" });
    expect(erreur.message).toBe("Session expiree — reconnecte-toi.");
    expect(erreur.status).toBe(401);
    expect(erreur.body).toBe("nope");
  });
});
