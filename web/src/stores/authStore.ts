import { defineStore } from "pinia";
import { ref } from "vue";
import { authService } from "@/services/authService";
import { Store as KvStore } from "@/api/store";
import { getDiscordToken } from "@/api/config";
import { HttpError } from "@/api/httpError";
import { httpGet, logoutSession, tryRefreshSession } from "@/api/http";
import type { DiscordUser } from "@/api/config";

const STORE_FILE = "auth.json";
const USER_KEY = "discord_user";

async function getKv() { return KvStore.load(STORE_FILE); }

export const useAuthStore = defineStore("auth", () => {
  const user = ref<DiscordUser | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const initialized = ref(false);

  async function checkSession() {
    if (initialized.value) return;
    initialized.value = true;

    // Restore le user depuis le storage (rapide).
    try {
      const currentUser = authService.getCurrentUser();
      if (currentUser) {
        user.value = currentUser;
      } else {
        const store = await getKv();
        const stored = await store.get<DiscordUser>(USER_KEY);
        if (stored) user.value = stored;
      }
    } catch {
      // Pas de session locale.
    }

    // Plus de token Discord en sessionStorage (ex: navigateur ferme + rouvert).
    // On tente un refresh SILENCIEUX via le cookie de session httpOnly (POST
    // /auth/refresh) -> "rester connecte" sans re-validation Discord. On evite
    // ce refresh sur /auth/* (le callback gere son propre cycle de vie et pose
    // le token lui-meme).
    if (!getDiscordToken() && !window.location.pathname.startsWith("/auth/")) {
      const ok = await tryRefreshSession();
      if (ok) {
        // tryRefreshSession a stocke le token + l'identite.
        user.value = authService.getCurrentUser();
      } else if (user.value) {
        // Aucune session serveur recuperable -> purge le user obsolete.
        user.value = null;
        try {
          const store = await getKv();
          await store.delete(USER_KEY);
        } catch { /* ignore */ }
        return;
      }
    }

    // Si on a un user en cache, valide que le token Discord est encore
    // accepte par l'API. Sur 401 (token expire) -> http.ts purge + redirige.
    // Sur 403 (le compte n'est plus dans SUPERADMIN_USER_IDS) on purge la
    // session locale et on renvoie vers /login avec un message explicite.
    if (user.value) {
      try {
        await httpGet("/api/auth/check-access");
        // check-access ne repond 200 qu'aux superadmins (le middleware refuse
        // les autres). Y arriver prouve donc le statut : on (re)pose le flag,
        // ce qui repare une identite en cache anterieure a son introduction.
        if (user.value && !user.value.is_superadmin) {
          user.value = { ...user.value, is_superadmin: true };
          try {
            const store = await getKv();
            await store.set(USER_KEY, user.value);
          } catch { /* ignore */ }
        }
      } catch (e) {
        if (e instanceof HttpError && e.status === 403) {
          authService.logout();
          user.value = null;
          try {
            const store = await getKv();
            await store.delete(USER_KEY);
          } catch { /* ignore */ }
          try {
            await logoutSession();
          } catch { /* best-effort */ }
          // Redirect manuel vers login avec message explicite.
          if (window.location.pathname !== "/login") {
            window.location.href = "/login?error=not_authorized";
          }
        }
        // 401 / network : http.ts gere ou on laisse passer.
      }
    }
  }

  async function login() {
    loading.value = true;
    error.value = null;
    try {
      const loggedUser = await authService.discordLogin();
      user.value = loggedUser;
      const store = await getKv();
      await store.set(USER_KEY, loggedUser);
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function logout() {
    // Supprime la session serveur + le cookie httpOnly (best-effort), puis purge local.
    try {
      await logoutSession();
    } catch { /* ignore */ }
    authService.logout();
    user.value = null;
    initialized.value = false;
    const store = await getKv();
    await store.delete(USER_KEY);
  }

  function avatarUrl(u: DiscordUser & { discriminator?: string }): string {
    if (u.avatar) {
      return `https://cdn.discordapp.com/avatars/${u.id}/${u.avatar}.png?size=64`;
    }
    const index = u.discriminator === "0"
      ? (BigInt(u.id) >> 22n) % 6n
      : Number(u.discriminator ?? 0) % 5;
    return `https://cdn.discordapp.com/embed/avatars/${index}.png`;
  }

  return {
    user, loading, error, initialized,
    checkSession, login, logout, avatarUrl,
  };
});
