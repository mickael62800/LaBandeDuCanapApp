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

import { OpsHttpError, opsDelete, opsGet, opsPatch, opsPost } from "./opsHttp";

describe("client exploitation (passerelle /ops-api)", () => {
  beforeEach(() => {
    // NB: on ne fait PAS mockClear sur createBackendClient — le module cree son client a l'import.
    mocks.request.mockReset().mockResolvedValue({ ok: true });
  });

  it("configure le client sur /ops-api avec les messages d'exploitation", () => {
    expect(mocks.createBackendClient).toHaveBeenCalledTimes(1);
    const options = mocks.createBackendClient.mock.calls[0][0];
    expect(options.baseUrl).toBe("/ops-api");
    expect(options.errorLabel).toBe("exploitation");
    expect(typeof options.forbiddenMessage).toBe("string");
    expect(typeof options.unavailableMessage).toBe("string");
    expect(typeof options.makeError).toBe("function");
  });

  it("opsGet envoie GET sans corps", async () => {
    await opsGet("/api/x");
    expect(mocks.request).toHaveBeenCalledWith("GET", "/api/x");
  });

  it.each([
    ["PATCH", "opsPatch"],
    ["POST", "opsPost"],
    ["DELETE", "opsDelete"],
  ] as const)("%s envoie le corps (%s)", async (method, label) => {
    const fn = { opsPatch, opsPost, opsDelete }[label];

    await fn("/api/z", { a: 1 });
    expect(mocks.request).toHaveBeenCalledWith(method, "/api/z", { body: { a: 1 } });

    // corps optionnel : undefined quand absent
    mocks.request.mockClear();
    await fn("/api/w");
    expect(mocks.request).toHaveBeenLastCalledWith(method, "/api/w", { body: undefined });
  });

  it("propage les erreurs du transport commun", async () => {
    const erreur = new Error("503");
    mocks.request.mockRejectedValue(erreur);
    await expect(opsGet("/api/x")).rejects.toBe(erreur);
  });
});

describe("OpsHttpError (identite d'erreur propre a l'exploitation)", () => {
  it("est une instance de Error avec le nom attendu et les champs details", () => {
    const erreur = new OpsHttpError("Acces refuse.", { status: 403, body: null });

    expect(erreur).toBeInstanceOf(Error);
    expect(erreur.name).toBe("OpsHttpError");
    expect(erreur.message).toBe("Acces refuse.");
    expect(erreur.status).toBe(403);
    expect(erreur.body).toBeNull();
  });

  it("conserve le statut dans les champs individuels", () => {
    const erreur = new OpsHttpError("Service indisponible.", { status: 503, body: "nope" });
    expect(erreur.message).toBe("Service indisponible.");
    expect(erreur.status).toBe(503);
    expect(erreur.body).toBe("nope");
  });

  it("makeError du client produit bien une OpsHttpError", () => {
    const options = mocks.createBackendClient.mock.calls[0][0];
    const erreur = (options.makeError as (m: string, d: unknown) => Error)("Boom.", { status: 502, body: null });

    expect(erreur).toBeInstanceOf(OpsHttpError);
    expect((erreur as OpsHttpError).status).toBe(502);
  });
});
