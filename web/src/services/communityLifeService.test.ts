import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  httpDelete: vi.fn(),
  httpGet: vi.fn(),
  httpPost: vi.fn(),
}));

vi.mock("@/api/http", () => mocks);

// `query` est une fonction pure : on la reimplemente telle quelle DANS la fabrique
// (auto-suffisante, aucune reference externe — exigence du hoisting de vi.mock).
const publicGet = vi.hoisted(() => vi.fn());
vi.mock("./publicHttp", () => ({
  publicGet: (...args: unknown[]) => publicGet(...(args as [])),
  query(params: Record<string, string | number | undefined>): string {
    const pairs = Object.entries(params)
      .filter(([, v]) => v !== undefined && v !== "")
      .map(([k, v]) => `${k}=${encodeURIComponent(String(v))}`);
    return pairs.length ? `?${pairs.join("&")}` : "";
  },
}));

const config = vi.hoisted(() => ({ getDiscordToken: vi.fn(() => "") }));
vi.mock("@/api/config", () => config);

import { communityActionsService, communityLifeService } from "./communityLifeService";
import type { Presence } from "./communityLifeService";

describe("communityLifeService (lectures publiques)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
    publicGet.mockReset().mockResolvedValue([] as never);
    config.getDiscordToken.mockReturnValue("");
  });

  it("lfg lit les annonces publiques avec la limite par defaut", async () => {
    const posts = [{ id: "a1" }];
    publicGet.mockResolvedValue(posts as never);

    await expect(communityLifeService.lfg("g1")).resolves.toEqual(posts);
    // `encodeURIComponent` : un identifiant special est encode.
    expect(publicGet).toHaveBeenCalledWith("/lfg/g1?limit=6");
  });

  it("polls lit les sondages ouverts", async () => {
    publicGet.mockResolvedValue([] as never);

    await communityLifeService.polls("g/2").then((r) => expect(r).toEqual([]));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(publicGet).toHaveBeenCalledWith("/polls/g%2F2?limit=3");
  });

  it("spotlight renvoie null tant que le staff n'a designe personne", async () => {
    publicGet.mockResolvedValue(null as never);

    await expect(communityLifeService.spotlight("g1")).resolves.toBeNull();
    // `encodeURIComponent` : un identifiant special est encode.
    expect(publicGet).toHaveBeenCalledWith("/spotlight/g1");
  });

  it("pulse lit anniversaires et nouveaux venus", async () => {
    const pulse = { anniversaries: [], newcomers: [{ username: "neo" }] };
    publicGet.mockResolvedValue(pulse as never);

    await expect(communityLifeService.pulse("g1")).resolves.toEqual(pulse);
    // `encodeURIComponent` : un identifiant special est encode.
    expect(publicGet).toHaveBeenCalledWith("/pulse/g1");
  });

  it("news lit les annonces publiees", async () => {
    publicGet.mockResolvedValue([] as never);

    await communityLifeService.news("g1").then((r) => expect(r).toEqual([]));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(publicGet).toHaveBeenCalledWith("/news/g1?limit=3");
  });

  describe("presence", () => {
    const presence: Presence = { voice: [], voice_total: 0, text: [] };

    it("sans token Discord : lecture publique directe", async () => {
      publicGet.mockResolvedValue(presence as never);

      await expect(communityLifeService.presence("g1")).resolves.toEqual(presence);
      // `encodeURIComponent` : un identifiant special est encode.
      expect(publicGet).toHaveBeenCalledWith("/presence/g1");
      expect(mocks.httpGet).not.toHaveBeenCalled();
    });

    it("avec token : passe par la surface authentifiee (salons reserves inclus)", async () => {
      config.getDiscordToken.mockReturnValue("tok-123");
      mocks.httpGet.mockResolvedValue(presence);

      await expect(communityLifeService.presence("g1")).resolves.toEqual(presence);
      // `encodeURIComponent` : un identifiant special est encode.
      expect(mocks.httpGet).toHaveBeenCalledWith("/api/presence/g1");
      expect(publicGet).not.toHaveBeenCalled();
    });

    it("avec token mais session expirée : repli sur la vue publique", async () => {
      config.getDiscordToken.mockReturnValue("tok-123");
      mocks.httpGet.mockRejectedValue(new Error("401"));
      publicGet.mockResolvedValue(presence as never);

      await expect(communityLifeService.presence("g1")).resolves.toEqual(presence);
      // `encodeURIComponent` : un identifiant special est encode.
      expect(publicGet).toHaveBeenCalledWith("/presence/g1");
    });
  });
});

describe("communityActionsService (ecritures, session requise)", () => {
  beforeEach(() => {
    for (const m of Object.values(mocks)) m.mockReset();
  });

  it("createLfg publie une annonce de recherche", async () => {
    const input = { game: "WoW", slots: 3 };
    mocks.httpPost.mockResolvedValue({ id: "l9" });

    await expect(communityActionsService.createLfg("g1", input)).resolves.toEqual({
      id: "l9",
    });
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/lfg/g1", input);
  });

  it("closeLfg ferme l'annonce sans la supprimer", async () => {
    mocks.httpPost.mockResolvedValue({ ok: true });

    await communityActionsService.closeLfg("l1").then((r) => expect(r).toEqual({ ok: true }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/lfg/detail/l1/close", {});
  });

  it("deleteLfg supprime l'annonce definitivement", async () => {
    mocks.httpDelete.mockResolvedValue({ deleted: true });

    await communityActionsService.deleteLfg("l1").then((r) => expect(r).toEqual({ deleted: true }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/lfg/detail/l1");
  });

  it("joinLfg dit « je viens » et renvoie l'annonce relue", async () => {
    mocks.httpPost.mockResolvedValue({ id: "l1" });

    await communityActionsService.joinLfg("l1").then((r) => expect(r).toEqual({ id: "l1" }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/lfg/detail/l1/join", {});
  });

  it("leaveLfg retire sa participation et renvoie l'annonce relue", async () => {
    mocks.httpDelete.mockResolvedValue({ id: "l1" });

    await communityActionsService.leaveLfg("l1").then((r) => expect(r).toEqual({ id: "l1" }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpDelete).toHaveBeenCalledWith("/api/lfg/detail/l1/join");
  });

  it("vote enregistre le choix d'une option", async () => {
    mocks.httpPost.mockResolvedValue({ id: "p1" });

    await communityActionsService.vote("p1", "o2").then((r) => expect(r).toEqual({ id: "p1" }));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpPost).toHaveBeenCalledWith("/api/polls/detail/p1/vote", { option_id: "o2" });
  });

  it("myPolls lit les sondages via /me (viewer non requis)", async () => {
    mocks.httpGet.mockResolvedValue([]);

    await communityActionsService.myPolls("g1").then((r) => expect(r).toEqual([]));
    // `encodeURIComponent` : un identifiant special est encode.
    expect(mocks.httpGet).toHaveBeenCalledWith("/api/me/polls/g1");
  });
});
