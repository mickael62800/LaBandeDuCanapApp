import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Le transport commun est isole : on verifie uniquement l'adaptation faite par
// `createBackendClient` (URL, en-tetes, defauts, mapping d'erreurs).
const mocks = vi.hoisted(() => ({ requestJson: vi.fn() }));
vi.mock("./httpTransport", async (importOriginal) => {
  const actual = await importOriginal<typeof import("./httpTransport")>();
  return { ...actual, requestJson: (...args: unknown[]) => mocks.requestJson(...(args as [])) };
});

// `tryRefreshSession` / `handleUnauthorizedSession` sont re-exportes tels quels :
// on verifie que le client les branche bien sur chaque requete.
import { handleUnauthorizedSession, tryRefreshSession } from "./http";
import { BackendHttpError, createBackendClient } from "./backendHttp";
import type { HttpErrorDetails } from "./httpError";

function memoryStorage(): Storage {
  const values = new Map<string, string>();
  return {
    get length() { return values.size; },
    clear: () => values.clear(),
    getItem: (key) => values.get(key) ?? null,
    key: (index) => [...values.keys()][index] ?? null,
    removeItem: (key) => { values.delete(key); },
    setItem: (key, value) => { values.set(key, String(value)); },
  };
}

const OPTIONS = {
  baseUrl: "/test-api",
  errorLabel: "TestBackend",
  forbiddenMessage: "Accès au backend de test refusé.",
};

describe("createBackendClient (adaptateur metier commun)", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.stubGlobal("localStorage", memoryStorage());
    vi.stubGlobal("sessionStorage", memoryStorage());
    mocks.requestJson.mockReset().mockResolvedValue({ data: "ok" });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("compose l'URL, les en-tetes et le contexte de requete attendus", async () => {
    sessionStorage.setItem("ds.discord.token", "tok-123");
    const signal = new AbortController().signal;
    const request = createBackendClient({ ...OPTIONS });

    await request("POST", "/items/7", { body: { a: 1 }, headers: { "X-Custom": "v" }, signal, timeoutMs: 4200 });

    expect(mocks.requestJson).toHaveBeenCalledTimes(1);
    const options = mocks.requestJson.mock.calls[0][0];
    expect(options.url).toBe("/test-api/items/7");
    expect(options.method).toBe("POST");
    expect(options.body).toEqual({ a: 1 });
    expect(options.signal).toBe(signal);
    expect(options.timeoutMs).toBe(4200);
    expect(options.credentials).toBe("include");
    expect(options.backend).toBe("TestBackend");

    const headers = options.headers();
    expect(headers["Content-Type"]).toBe("application/json");
    expect(headers["X-Custom"]).toBe("v");
    expect(headers["X-Discord-Token"]).toBe("tok-123");
  });

  it("omet le jeton Discord quand il est absent", async () => {
    const request = createBackendClient({ ...OPTIONS });
    await request("GET", "/items");

    const options = mocks.requestJson.mock.calls[0][0];
    expect(options.headers()["X-Discord-Token"]).toBeUndefined();
  });

  it("applique les defauts emptyStatuses [204] et retryStatuses [503]", async () => {
    const request = createBackendClient({ ...OPTIONS, makeError: (m: string) => new Error(m) });
    await request("GET", "/items");

    const options = mocks.requestJson.mock.calls[0][0];
    expect(options.emptyStatuses).toEqual(new Set([204]));
    expect(options.retryStatuses).toEqual(new Set([503]));
  });

  it("honore emptyStatuses et retryStatuses fournis", async () => {
    const request = createBackendClient({ ...OPTIONS, emptyStatuses: [201, 202], retryStatuses: [429, 502] });
    await request("GET", "/items");

    const options = mocks.requestJson.mock.calls[0][0];
    expect(options.emptyStatuses).toEqual(new Set([201, 202]));
    expect(options.retryStatuses).toEqual(new Set([429, 502]));
  });

  it("branche refreshSession et onUnauthorized du module http", async () => {
    const request = createBackendClient({ ...OPTIONS, makeError: (m: string) => new Error(m) });
    await request("GET", "/items");

    const options = mocks.requestJson.mock.calls[0][0];
    expect(options.refreshSession).toBe(tryRefreshSession);
    expect(options.onUnauthorized).toBe(handleUnauthorizedSession);
  });

  it("propage les donnees du transport a l'appelant", async () => {
    const payload = [{ id: 1 }];
    mocks.requestJson.mockResolvedValue({ data: payload, response: new Response() });
    const request = createBackendClient({ ...OPTIONS, makeError: (m: string) => new Error(m) });

    await expect(request("GET", "/items")).resolves.toBe(payload);
  });

  it("propage les erreurs du transport telles quelles", async () => {
    const erreur = new BackendHttpError("boom", { status: 500, body: null });
    mocks.requestJson.mockRejectedValue(erreur);
    const request = createBackendClient({ ...OPTIONS, makeError: (m: string) => new Error(m) });

    await expect(request("GET", "/items")).rejects.toBe(erreur);
  });

  describe("mapping des messages d'erreur par statut", () => {
    // Simule le transport commun : sur une reponse non ok, `requestJson` appelle
    // makeError(messageDuCorps ?? "Erreur <status>", details). On capture ensuite
    // le message visible produit par l'adaptateur.
    async function erreurPour(
      status: number,
      opts?: { bodyMessage?: string; unavailableMessage?: string },
    ): Promise<string> {
      let visible = "";
      const details: HttpErrorDetails = { status, body: null };
      mocks.requestJson.mockImplementation(async (options) => {
        throw options.makeError(opts?.bodyMessage ?? `Erreur ${status}`, details);
      });
      const request = createBackendClient({
        ...OPTIONS,
        unavailableMessage: opts?.unavailableMessage,
        makeError: (message: string) => { visible = message; return new BackendHttpError(message, details); },
      });

      await expect(request("GET", "/x")).rejects.toBeDefined();
      return visible;
    }

    it("401 -> message de session expiree (prioritaire)", async () => {
      const message = await erreurPour(401);
      expect(message).toBe("Session expirée — reconnecte-toi.");
    });

    it("403 -> forbiddenMessage du backend", async () => {
      const message = await erreurPour(403, { unavailableMessage: "Service hors ligne." });
      expect(message).toBe("Accès au backend de test refusé.");
    });

    it("502/503 avec unavailableMessage -> ce dernier est affiche", async () => {
      for (const status of [502, 503]) {
        const message = await erreurPour(status, { unavailableMessage: "Service hors ligne." });
        expect(message).toBe("Service hors ligne.");
      }
    });

    it("502/503 sans unavailableMessage -> libelle generique du backend", async () => {
      for (const status of [502, 503]) {
        const message = await erreurPour(status);
        expect(message).toBe(`Erreur TestBackend (${status})`);
      }
    });

    it("autre statut sans corps -> libelle generique du backend", async () => {
      const message = await erreurPour(500, { unavailableMessage: "Service hors ligne." });
      expect(message).toBe("Erreur TestBackend (500)");
    });

    it("conserve un message metier deja present dans le corps", async () => {
      const message = await erreurPour(422, { bodyMessage: "Valeur impossible" });
      expect(message).toBe("Valeur impossible");
    });
  });

  it("BackendHttpError herite de HttpError avec un nom parametrisable", async () => {
    const actual = await import("./httpError");
    const erreur = new BackendHttpError("msg", { status: 418, body: "teapot" }, "MonNom");

    expect(erreur).toBeInstanceOf(actual.HttpError);
    expect(erreur.name).toBe("MonNom");
    expect(erreur.status).toBe(418);
    expect(erreur.body).toBe("teapot");

    const defaut = new BackendHttpError("msg", { status: 500, body: null });
    expect(defaut.name).toBe("BackendHttpError");
  });
});
