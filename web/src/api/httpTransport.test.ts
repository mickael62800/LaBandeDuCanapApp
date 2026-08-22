import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { HttpError, HttpTimeoutError } from "./httpError";
import { fetchWithTimeout, requestJson } from "./httpTransport";
import { tryRefreshSession } from "./http";

function jsonResponse(body: unknown, status = 200, headers?: HeadersInit): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json", ...headers },
  });
}

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

describe("requestJson", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    vi.stubGlobal("localStorage", memoryStorage());
    vi.stubGlobal("sessionStorage", memoryStorage());
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("retourne les données et conserve les en-têtes de réponse", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
      jsonResponse([{ id: 1 }], 200, { "X-Total-Count": "42" }),
    ));

    const result = await requestJson<Array<{ id: number }>>({
      url: "/api/items",
      method: "GET",
    });

    expect(result.data).toEqual([{ id: 1 }]);
    expect(result.response.headers.get("X-Total-Count")).toBe("42");
  });

  it("produit une HttpError structurée sans perdre le message métier", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({
      error: "Données invalides : Valeur impossible",
      code: "invalid_value",
      request_id: "req-body",
    }, 403, { "X-Request-Id": "req-header" })));

    const error = await requestJson({
      url: "/api/items",
      method: "POST",
      backend: "Sentinel",
    }).catch((reason: unknown) => reason);

    expect(error).toBeInstanceOf(HttpError);
    expect(error).toMatchObject({
      message: "Valeur impossible",
      status: 403,
      code: "invalid_value",
      requestId: "req-header",
      backend: "Sentinel",
      body: {
        error: "Données invalides : Valeur impossible",
        code: "invalid_value",
        request_id: "req-body",
      },
    });
  });

  it("borne le corps d'une erreur non JSON", async () => {
    const longBody = "x".repeat(300);
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(longBody, { status: 502 })));

    await expect(requestJson({ url: "/proxy", method: "GET" }))
      .rejects.toMatchObject({ status: 502, message: "x".repeat(200) });
  });

  it("rafraîchit puis rejoue une seule fois une requête 401", async () => {
    let token = "old";
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({ error: "expired" }, 401))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);
    const refreshSession = vi.fn(async () => {
      token = "new";
      return true;
    });

    const { data } = await requestJson<{ ok: boolean }>({
      url: "/api/items",
      method: "GET",
      headers: () => ({ Authorization: `Bearer ${token}` }),
      refreshSession,
    });

    expect(data.ok).toBe(true);
    expect(refreshSession).toHaveBeenCalledOnce();
    expect(fetchMock).toHaveBeenCalledTimes(2);
    expect(fetchMock.mock.calls[1]?.[1]).toMatchObject({
      headers: { Authorization: "Bearer new" },
    });
  });

  it("signale la perte de session quand le refresh échoue", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse({ error: "expired" }, 401)));
    const onUnauthorized = vi.fn();

    await expect(requestJson({
      url: "/api/items",
      method: "GET",
      refreshSession: async () => false,
      onUnauthorized,
    })).rejects.toMatchObject({ status: 401 });
    expect(onUnauthorized).toHaveBeenCalledOnce();
  });

  it("retente les GET 503 et respecte Retry-After", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({ error: "busy" }, 503, { "Retry-After": "0" }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(requestJson<{ ok: boolean }>({ url: "/api/items", method: "GET" }))
      .resolves.toMatchObject({ data: { ok: true } });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("retente un GET 404 seulement quand le backend le declare transitoire", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({ error: "ancienne route" }, 404, { "Retry-After": "0" }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(requestJson<{ ok: boolean }>({
      url: "/atrium-api/admin/guilds/1/config",
      method: "GET",
      retryStatuses: new Set([404, 502, 503]),
    })).resolves.toMatchObject({ data: { ok: true } });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });


  it("retente les GET 503 en suivant un Retry-After exprime en date", async () => {
    const future = new Date(Date.now() + 2_000).toUTCString();
    const fetchMock = vi.fn()
      .mockResolvedValueOnce(jsonResponse({ error: "busy" }, 503, { "Retry-After": future }))
      .mockResolvedValueOnce(jsonResponse({ ok: true }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(requestJson<{ ok: boolean }>({ url: "/api/items", method: "GET" }))
      .resolves.toMatchObject({ data: { ok: true } });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });

  it("epuise les trois tentatives puis propage la derniere reponse", async () => {
    const fetchMock = vi.fn()
      .mockResolvedValue(jsonResponse({ error: "busy" }, 503, { "Retry-After": "0" }));
    vi.stubGlobal("fetch", fetchMock);

    await expect(requestJson({ url: "/api/items", method: "GET" }))
      .rejects.toMatchObject({ status: 503 });
    expect(fetchMock).toHaveBeenCalledTimes(3);
  });

  it("interrompt la rejeu quand le signal est deja annule", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(
      jsonResponse({ error: "busy" }, 503, { "Retry-After": "0" }),
    ));
    const controller = new AbortController();
    controller.abort(new DOMException("Aborted", "AbortError"));

    await expect(requestJson({ url: "/api/items", method: "GET", signal: controller.signal }))
      .rejects.toBeInstanceOf(DOMException);
  });

  it("interrompt la pause de retry si le signal est annule pendant l'attente", async () => {
    // Le delai d'attente utilise window.setTimeout : on intercepte les callbacks
    // pour rejouer manuellement la fin du timer, sans attendre reellement.
    const timers: Array<() => void> = [];
    vi.stubGlobal("setTimeout", (cb: () => void) => { timers.push(cb); return 1; });
    vi.stubGlobal("clearTimeout", () => undefined);

    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValueOnce(jsonResponse({ error: "busy" }, 503, { "Retry-After": "2" })),
    );
    const controller = new AbortController();

    const promesse = requestJson<{ ok: boolean }>({ url: "/api/items", method: "GET", signal: controller.signal });
    // La premiere reponse est en attente de retry : on annule pendant la pause.
    await Promise.resolve();
    controller.abort(new DOMException("Aborted", "AbortError"));

    await expect(promesse).rejects.toBeInstanceOf(DOMException);
  });

  it("propage l'erreur reseau sans la convertir en timeout", async () => {
    const networkError = new TypeError("Failed to fetch");
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(networkError));

    await expect(fetchWithTimeout("/api/down", {}, 10_000))
      .rejects.toBe(networkError);
  });

  it("retourne data undefined pour un statut vide declare", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 204 })));

    const { data } = await requestJson<unknown>({
      url: "/api/items/1",
      method: "DELETE",
      emptyStatuses: new Set([204]),
    });

    expect(data).toBeUndefined();
  });

  it("ne retente pas un 404 par defaut", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({ error: "absent" }, 404));
    vi.stubGlobal("fetch", fetchMock);

    await expect(requestJson({ url: "/api/absent", method: "GET" }))
      .rejects.toMatchObject({ status: 404 });
    expect(fetchMock).toHaveBeenCalledOnce();
  });

  it("convertit un dépassement de délai en HttpTimeoutError", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("fetch", vi.fn((_input: RequestInfo | URL, init?: RequestInit) =>
      new Promise<Response>((_resolve, reject) => {
        init?.signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")));
      })));

    const result = expect(fetchWithTimeout("/slow", {}, 25))
      .rejects.toBeInstanceOf(HttpTimeoutError);
    await vi.advanceTimersByTimeAsync(25);
    await result;
  });

  it("déduplique les refreshs de session concurrents", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse({
      token: "fresh-token",
      id: "1",
      username: "nexus",
      is_superadmin: true,
    }));
    vi.stubGlobal("fetch", fetchMock);

    const [first, second] = await Promise.all([
      tryRefreshSession(),
      tryRefreshSession(),
    ]);

    expect(first).toBe(true);
    expect(second).toBe(true);
    expect(fetchMock).toHaveBeenCalledOnce();
    expect(sessionStorage.getItem("ds.discord.token")).toBe("fresh-token");
  });
});
