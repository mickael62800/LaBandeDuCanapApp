<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { setDiscordUser, setDiscordToken, type DiscordUser } from "@/api/config";
import { useAuthStore } from "@/stores/authStore";
import { httpGet } from "@/api/http";
import { takeEntryDestination } from "@/entrySpace";

const router = useRouter();
const store = useAuthStore();

const status = ref<"redeeming" | "ok" | "error">("ok");
const message = ref("Connexion en cours…");

onMounted(async () => {
  // Backend redirige ici avec les infos dans le FRAGMENT (#…) pour eviter
  // que le token n'apparaisse dans les logs serveur ou le referer.
  const hash = window.location.hash.startsWith("#")
    ? window.location.hash.slice(1)
    : window.location.hash;
  const params = new URLSearchParams(hash);

  const token = params.get("token");
  const id = params.get("id");
  const username = params.get("username");

  if (!token || !id || !username) {
    router.replace({ name: "login", query: { error: "callback_invalide" } });
    return;
  }

  const user: DiscordUser = {
    id,
    username,
    global_name: params.get("global_name") || null,
    avatar: params.get("avatar") || null,
    is_superadmin: params.get("is_superadmin") === "1",
  };

  setDiscordToken(token);
  setDiscordUser(user);

  // Nettoie l'URL (retire le fragment sensible) avant la prochaine nav.
  history.replaceState(null, "", window.location.pathname);

  // Verifie que le compte Discord est autorise. L'acces au back-office est
  // reserve aux identifiants listes dans SUPERADMIN_USER_IDS cote API : celle-ci
  // repond 200 si c'est le cas, 403 sinon.
  status.value = "redeeming";
  message.value = "Vérification des accès…";
  try {
    await httpGet("/api/auth/check-access");
  } catch (e) {
    const msg = errMsg(e);
    if (msg.includes("403")) {
      message.value = "Accès refusé. Ce compte Discord n'est pas administrateur.";
      status.value = "error";
      // Cleanup le token pour ne pas laisser une session "fantome"
      setDiscordToken("");
      setDiscordUser(null);
      store.$patch({ user: null, initialized: true });
      await new Promise((r) => setTimeout(r, 2500));
      router.replace({ name: "login", query: { error: "not_authorized" } });
      return;
    }
    // Erreur transitoire (reseau, rate limit Discord) : on laisse passer
    // plutot que de bloquer l'administrateur legitime.
    console.warn("check-access failed, proceeding anyway:", e);
  }

  // Injecte directement dans le store Pinia pour eviter un re-check async.
  store.$patch({ user, initialized: true, error: null });

  // Membre ou administration selon le bouton d'origine.
  router.replace(takeEntryDestination());
});
</script>

<template>
  <div class="callback-page">
    <div class="callback-card">
      <div class="spinner" :class="status"></div>
      <p :class="['message', status]">{{ message }}</p>
    </div>
  </div>
</template>

<style scoped>
.callback-page {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--bg-primary), var(--bg-secondary));
}
.callback-card {
  text-align: center;
  padding: 32px;
  max-width: 420px;
}
.spinner {
  width: 48px;
  height: 48px;
  margin: 0 auto 20px;
  border: 4px solid var(--bg-secondary);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}
.spinner.error {
  border-top-color: var(--danger);
  animation-duration: 1.5s;
}
.spinner.ok {
  border-top-color: var(--success, var(--success));
}
@keyframes spin {
  to { transform: rotate(360deg); }
}
.message {
  font-size: 14px;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.5;
}
.message.error { color: var(--danger); }
.message.ok { color: var(--text-primary); }
</style>
