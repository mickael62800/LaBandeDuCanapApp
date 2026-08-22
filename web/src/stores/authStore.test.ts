import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const mocks = vi.hoisted(() => ({
  getCurrentUser: vi.fn(),
  discordLogin: vi.fn(),
  logout: vi.fn(),
  kvLoad: vi.fn(),
  getDiscordToken: vi.fn().mockReturnValue(null),
  httpGet: vi.fn(),
  logoutSession: vi.fn(),
  tryRefreshSession: vi.fn(),
}));

vi.mock("@/services/authService", () => ({
  authService: {
    getCurrentUser: mocks.getCurrentUser,
    discordLogin: mocks.discordLogin,
    logout: mocks.logout,
  },
}));

vi.mock("@/api/store", () => ({ Store: { load: mocks.kvLoad } }));

vi.mock("@/api/config", () => ({ getDiscordToken: mocks.getDiscordToken }));

vi.mock("@/api/http", () => ({
  httpGet: mocks.httpGet,
  logoutSession: mocks.logoutSession,
  tryRefreshSession: mocks.tryRefreshSession,
}));

import { HttpError } from "@/api/httpError";
import type { DiscordUser } from "@/types";
import { useAuthStore } from "./authStore";

const USER_KEY = "discord_user";

function fakeKv(over?: Record<string, unknown>) {
  return {
    get: vi.fn().mockResolvedValue(null),
    set: vi.fn().mockResolvedValue(undefined),
    delete: vi.fn().mockResolvedValue(undefined),
    ...over,
  };
}

const baseUser = (extra?: Partial<DiscordUser>): DiscordUser => ({
  id: "1",
  username: "micka",
  avatar: null,
  global_name: "Mick",
  is_superadmin: false,
  ...extra,
});

describe("useAuthStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    window.history.pushState({}, "", "/");
    for (const m of Object.values(mocks)) m.mockReset();
    mocks.getDiscordToken.mockReturnValue(null);
    mocks.kvLoad.mockResolvedValue(fakeKv());
  });

  describe("checkSession", () => {
    it("restaure l'utilisateur local, valide le token et repare le flag superadmin", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.tryRefreshSession.mockResolvedValue(true);
      mocks.httpGet.mockResolvedValue({ ok: true });

      await useAuthStore().checkSession();

      expect(mocks.httpGet).toHaveBeenCalledWith("/api/auth/check-access");
      const store = useAuthStore();
      expect(store.user?.is_superadmin).toBe(true);
      expect(kv.set).toHaveBeenCalledWith(USER_KEY, store.user);
    });

    it("ne reecrit pas le flag deja pose et saute le refresh si un token existe", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.getCurrentUser.mockReturnValue(baseUser({ is_superadmin: true }));
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.httpGet.mockResolvedValue({ ok: true });

      await useAuthStore().checkSession();

      expect(mocks.tryRefreshSession).not.toHaveBeenCalled();
      expect(kv.set).not.toHaveBeenCalled();
    });

    it("saute le refresh silencieux sur les routes /auth/*", async () => {
      window.history.pushState({}, "", "/auth/callback");
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.httpGet.mockResolvedValue({ ok: true });

      await useAuthStore().checkSession();

      expect(mocks.tryRefreshSession).not.toHaveBeenCalled();
      expect(useAuthStore().user?.is_superadmin).toBe(true);
    });

    it("restaure l'identite depuis le stockage quand authService est vide", async () => {
      const kv = fakeKv({ get: vi.fn().mockResolvedValue(baseUser()) });
      mocks.kvLoad.mockResolvedValue(kv);
      // Premier appel (restauration) : vide. Deuxieme (apres refresh) : identite fraiche.
      mocks.getCurrentUser
        .mockReturnValueOnce(null)
        .mockReturnValue(baseUser({ username: "fraichement" }));
      mocks.tryRefreshSession.mockResolvedValue(true);
      mocks.httpGet.mockResolvedValue({ ok: true });

      await useAuthStore().checkSession();

      expect(useAuthStore().user?.username).toBe("fraichement");
    });

    it("purge l'utilisateur obslete quand le refresh echoue", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.tryRefreshSession.mockResolvedValue(false);

      await useAuthStore().checkSession();

      expect(useAuthStore().user).toBeNull();
      expect(kv.delete).toHaveBeenCalledWith(USER_KEY);
      expect(mocks.httpGet).not.toHaveBeenCalled();
    });

    it("continue la purge meme si le stockage local echoue", async () => {
      mocks.kvLoad.mockRejectedValue(new Error("storage down"));
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.tryRefreshSession.mockResolvedValue(false);

      await expect(useAuthStore().checkSession()).resolves.toBeUndefined();

      expect(useAuthStore().user).toBeNull();
    });

    it("ignore l'echec de la restauration locale et continue sans user", async () => {
      const kv = fakeKv({ get: vi.fn().mockRejectedValue(new Error("corrompu")) });
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.getCurrentUser.mockReturnValue(null);
      mocks.tryRefreshSession.mockResolvedValue(false);

      await expect(useAuthStore().checkSession()).resolves.toBeUndefined();

      expect(useAuthStore().user).toBeNull();
    });

    it("ignore un echec d'ecriture du flag superadmin", async () => {
      const kv = fakeKv({ set: vi.fn().mockRejectedValue(new Error("plein")) });
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.httpGet.mockResolvedValue({ ok: true });

      await expect(useAuthStore().checkSession()).resolves.toBeUndefined();

      expect(useAuthStore().user?.is_superadmin).toBe(true);
    });

    it("sur 403 : logout complet, purge locale et redirection vers /login", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.httpGet.mockRejectedValue(
        new HttpError("refuse", { status: 403, code: "not_authorized" }),
      );

      await useAuthStore().checkSession();

      expect(mocks.logout).toHaveBeenCalled();
      expect(useAuthStore().user).toBeNull();
      expect(kv.delete).toHaveBeenCalledWith(USER_KEY);
      expect(mocks.logoutSession).toHaveBeenCalled();
      expect(window.location.href).toContain("/login?error=not_authorized");
    });

    it("sur 403 : ne redirige pas si deja sur /login", async () => {
      window.history.pushState({}, "", "/login");
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.httpGet.mockRejectedValue(
        new HttpError("refuse", { status: 403, code: "not_authorized" }),
      );

      await useAuthStore().checkSession();

      expect(window.location.href).not.toContain("error=not_authorized");
    });

    it("sur 403 : tolere les echecs de purge et de logout serveur", async () => {
      mocks.kvLoad.mockRejectedValue(new Error("storage down"));
      mocks.logoutSession.mockRejectedValue(new Error("reseau mort"));
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.httpGet.mockRejectedValue(
        new HttpError("refuse", { status: 403, code: "not_authorized" }),
      );

      await expect(useAuthStore().checkSession()).resolves.toBeUndefined();

      expect(mocks.logout).toHaveBeenCalled();
      expect(useAuthStore().user).toBeNull();
    });

    it("laisse passer une erreur 401 (geree par http.ts)", async () => {
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.getCurrentUser.mockReturnValue(baseUser());
      mocks.httpGet.mockRejectedValue(
        new HttpError("expire", { status: 401, code: "expired" }),
      );

      await expect(useAuthStore().checkSession()).resolves.toBeUndefined();

      // Pas de purge : http.ts a deja gere la redirection.
      expect(mocks.logout).not.toHaveBeenCalled();
    });

    it("ne se relance pas une deuxieme fois", async () => {
      mocks.getDiscordToken.mockReturnValue("token-discord");
      mocks.getCurrentUser.mockReturnValue(baseUser({ is_superadmin: true }));
      mocks.httpGet.mockResolvedValue({ ok: true });
      const store = useAuthStore();

      await store.checkSession();
      await store.checkSession();

      expect(mocks.httpGet).toHaveBeenCalledTimes(1);
    });
  });

  describe("login", () => {
    it("enregistre l'utilisateur connecte dans le stockage local", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      const connecte = baseUser({ username: "connecte" });
      mocks.discordLogin.mockResolvedValue(connecte);

      await useAuthStore().login();

      expect(useAuthStore().user).toEqual(connecte);
      expect(kv.set).toHaveBeenCalledWith(USER_KEY, connecte);
    });

    it("capture l'erreur de connexion et remet loading a false", async () => {
      const boom = new Error("refuse par Discord");
      mocks.discordLogin.mockRejectedValue(boom);

      await useAuthStore().login();

      expect(useAuthStore().error).toBe(String(boom));
      expect(useAuthStore().loading).toBe(false);
    });
  });

  describe("logout", () => {
    it("purge la session serveur, locale et le stockage", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      useAuthStore().user = baseUser();

      await useAuthStore().logout();

      expect(mocks.logoutSession).toHaveBeenCalled();
      expect(mocks.logout).toHaveBeenCalled();
      expect(useAuthStore().user).toBeNull();
      expect(kv.delete).toHaveBeenCalledWith(USER_KEY);
    });

    it("complete le logout local meme si la session serveur echoue", async () => {
      const kv = fakeKv();
      mocks.kvLoad.mockResolvedValue(kv);
      mocks.logoutSession.mockRejectedValue(new Error("reseau mort"));
      useAuthStore().user = baseUser();

      await expect(useAuthStore().logout()).resolves.toBeUndefined();

      expect(mocks.logout).toHaveBeenCalled();
      expect(useAuthStore().user).toBeNull();
    });
  });

  describe("avatarUrl", () => {
    it("utilise l'avatar Discord quand il existe", () => {
      const store = useAuthStore();

      expect(
        store.avatarUrl(baseUser({ id: "42", avatar: "abc123" })),
      ).toBe("https://cdn.discordapp.com/avatars/42/abc123.png?size=64");
    });

    it("calcule l'index par defaut sur le snowflake quand discriminator est 0", () => {
      const store = useAuthStore();

      expect(store.avatarUrl(baseUser({ id: "1" }))).toBe(
        `https://cdn.discordapp.com/embed/avatars/${((BigInt("1") >> 22n) % 6n).toString()}.png`,
      );
    });

    it("calcule l'index sur le discriminator legacy sinon", () => {
      const store = useAuthStore();

      expect(
        store.avatarUrl({ ...baseUser(), discriminator: "3" } as DiscordUser & { discriminator?: string }),
      ).toBe("https://cdn.discordapp.com/embed/avatars/3.png");
    });

    it("tombe sur l'index 0 sans avatar ni discriminator", () => {
      const store = useAuthStore();

      expect(store.avatarUrl(baseUser())).toBe(
        "https://cdn.discordapp.com/embed/avatars/0.png",
      );
    });
  });
});
