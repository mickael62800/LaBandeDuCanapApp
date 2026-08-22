import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ requestJson: vi.fn() }));

vi.mock("@/api/httpTransport", () => mocks);

import { anonymousJsonGet, publicGet, query } from "./publicHttp";

describe("anonymousJsonGet (GET JSON sans credential)", () => {
  beforeEach(() => {
    mocks.requestJson.mockReset().mockResolvedValue({ data: "ok" });
  });

  it("force credentials omit + Accept json et retourne le corps", async () => {
    const signal = new AbortController().signal;

    await expect(anonymousJsonGet("/nexus-public/x", { signal, timeoutMs: 1234 })).resolves.toBe("ok");

    expect(mocks.requestJson).toHaveBeenCalledWith({
      url: "/nexus-public/x",
      method: "GET",
      credentials: "omit",
      headers: expect.any(Function),
      signal,
      timeoutMs: 1234,
      backend: "Public",
    });

    const appel = mocks.requestJson.mock.calls[0][0];
    expect(appel.headers()).toEqual({ Accept: "application/json" });
  });

  it("propage les erreurs du transport (pas de refresh session)", async () => {
    const erreur = new Error("401");
    mocks.requestJson.mockRejectedValue(erreur);
    await expect(anonymousJsonGet("/x")).rejects.toBe(erreur);
  });
});

describe("publicGet", () => {
  it("prefixe le chemin par /api/public et propage les options", async () => {
    const signal = new AbortController().signal;
    mocks.requestJson.mockResolvedValue({ data: [1, 2] });

    await expect(publicGet("/events/g1")).resolves.toEqual([1, 2]);
    expect(mocks.requestJson).toHaveBeenCalledWith(
      expect.objectContaining({ url: "/api/public/events/g1" }),
    );
  });
});

describe("query (construction de query string)", () => {
  it("ignore les parametres absents ou vides", () => {
    expect(query({ a: "x", b: undefined, c: "" })).toBe("?a=x");
    expect(query({})).toBe("");
    expect(query({ a: undefined, b: "" })).toBe("");
  });

  it("encode les valeurs et accepte des nombres", () => {
    expect(query({ from: "2026-01-01T08:30:00.000Z", limit: 5 })).toBe(
      "?from=2026-01-01T08%3A30%3A00.000Z&limit=5",
    );
  });

  it("conserve l'ordre d'insertion des cles", () => {
    expect(query({ z: "1", a: "2" })).toBe("?z=1&a=2");
  });
});
