import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ publicGet: vi.fn() }));

vi.mock("./publicHttp", () => mocks);

import { isOngoing, publicEventsService, type PublicEvent } from "./publicEventsService";

describe("publicEventsService (planning public)", () => {
  beforeEach(() => {
    mocks.publicGet.mockReset().mockResolvedValue([]);
  });

  it("list envoie les bornes ISO encodees et la guilde", async () => {
    const from = new Date(Date.UTC(2026, 5, 14, 8, 30));
    const to = new Date(Date.UTC(2026, 7, 20, 22, 0));

    await publicEventsService.list("g/9", from, to);

    expect(mocks.publicGet).toHaveBeenCalledWith(
      `/events/g%2F9?from=${encodeURIComponent(from.toISOString())}&to=${encodeURIComponent(to.toISOString())}`,
    );
  });

  it("propage les erreurs du transport public", async () => {
    const erreur = new Error("503");
    mocks.publicGet.mockRejectedValue(erreur);
    await expect(publicEventsService.list("g1", new Date(), new Date())).rejects.toBe(erreur);
  });
});

describe("isOngoing (evenement en cours a cet instant)", () => {
  const base: PublicEvent = { id: "e1" };

  it("vrai quand now est dans la fenetre [starts_at, ends_at]", () => {
    const ev = { ...base, starts_at: new Date(Date.UTC(2026, 5, 14, 8)).toISOString(), ends_at: new Date(Date.UTC(2026, 5, 14, 23)).toISOString() };
    expect(isOngoing(ev, new Date(Date.UTC(2026, 5, 14, 12)))).toBe(true);

    // bornes inclusives : debut et fin exacts comptent comme en cours
    expect(isOngoing(ev, new Date(Date.UTC(2026, 5, 14, 8)))).toBe(true);
    expect(isOngoing(ev, new Date(Date.UTC(2026, 5, 14, 23)))).toBe(true);
  });

  it("faux avant le debut ou apres la fin", () => {
    const ev = { ...base, starts_at: "2026-07-01T08:00:00.000Z", ends_at: "2026-07-01T23:00:00.000Z" };
    expect(isOngoing(ev, new Date("2026-06-30T23:59:00.000Z"))).toBe(false);
    expect(isOngoing(ev, new Date("2026-07-02T00:01:00.000Z"))).toBe(false);
  });

  it("utilise la date courante quand aucun instant n'est fourni", () => {
    const maintenant = new Date();
    const ev = { ...base, starts_at: "2000-01-01T00:00:00.000Z", ends_at: "3000-01-01T00:00:00.000Z" };
    expect(isOngoing(ev)).toBe(true);

    const passe = { ...base, starts_at: "2000-01-01T00:00:00.000Z", ends_at: "2000-01-02T00:00:00.000Z" };
    expect(isOngoing(passe)).toBe(false);

    const futur = { ...base, starts_at: new Date(maintenant.getTime() + 60_000).toISOString(), ends_at: "3000-01-01T00:00:00.000Z" };
    expect(isOngoing(futur)).toBe(false);
  });
});
