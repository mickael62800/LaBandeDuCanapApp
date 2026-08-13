<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { setDiscordUser, setDiscordToken, type DiscordUser } from "@/api/config";
import { useAuthStore } from "@/stores/authStore";
import { httpGet } from "@/api/http";
import { HttpError } from "@/api/httpError";
import { takeEntryDestination } from "@/entrySpace";

const router = useRouter();
const store = useAuthStore();

const status = ref<"redeeming" | "ok" | "error">("ok");
const message = ref("Connexion en cours…");
/// Une panne reseau ou un 5xx est transitoire : l'utilisateur doit pouvoir
/// relancer la verification sans repasser par tout le tour OAuth.
const reessayable = ref(false);
/// Conserve entre deux tentatives : le fragment d'URL a ete efface juste apres
/// lecture, on ne peut plus le relire.
let utilisateur: DiscordUser | null = null;

/// Termine la session locale et ouvre le back-office. Appele UNIQUEMENT apres
/// un `check-access` en succes.
///
/// L'identite n'est persistee qu'ici, et c'est le coeur du correctif : elle
/// l'etait auparavant AVANT la verification. `authStore.checkSession` relit
/// cette valeur au demarrage — un echec de `check-access` laissait donc, au
/// rechargement suivant, une session locale que plus rien ne confrontait a
/// l'API.
function ouvrirLaSession(user: DiscordUser) {
  setDiscordUser(user);
  store.$patch({ user, initialized: true, error: null });
  router.replace(takeEntryDestination());
}

/// Efface tout ce qui pourrait passer pour une session.
async function refuser(erreurLogin: string, texte: string) {
  message.value = texte;
  status.value = "error";
  reessayable.value = false;
  setDiscordToken("");
  setDiscordUser(null);
  store.$patch({ user: null, initialized: true });
  await new Promise((r) => setTimeout(r, 2500));
  router.replace({ name: "login", query: { error: erreurLogin } });
}

async function verifierLAcces() {
  if (!utilisateur) return;
  status.value = "redeeming";
  reessayable.value = false;
  message.value = "Vérification des accès…";

  try {
    await httpGet("/api/auth/check-access");
  } catch (e) {
    // 403 : reponse claire de l'API, ce compte n'est pas administrateur.
    const refuse = e instanceof HttpError ? e.status === 403 : errMsg(e).includes("403");
    if (refuse) {
      await refuser(
        "not_authorized",
        "Accès refusé. Ce compte Discord n'est pas administrateur.",
      );
      return;
    }

    // TOUT LE RESTE echoue en se FERMANT.
    //
    // Ce chemin laissait passer : le profil issu du fragment OAuth etait place
    // dans Pinia et les routes d'administration devenaient navigables, sur un
    // simple `console.warn`. Les API restaient protegees cote serveur — ce
    // point ne donnait donc pas acces aux donnees a lui seul — mais il affichait
    // une interface privilegiee avant validation, et transformait toute route
    // backend oubliee en fuite. Une defense en profondeur qui s'ouvre a la
    // premiere panne reseau n'en est pas une.
    //
    // Le jeton reste en sessionStorage : c'est lui qui permet de reessayer sans
    // refaire le tour OAuth. Aucune identite n'est persistee, donc rien ne
    // ressemble a une session ouverte.
    message.value =
      "Vérification des accès impossible : le serveur n'a pas répondu. " +
      "Vos droits n'ont pas pu être confirmés.";
    status.value = "error";
    reessayable.value = true;
    return;
  }

  ouvrirLaSession(utilisateur);
}

function retourConnexion() {
  setDiscordToken("");
  setDiscordUser(null);
  store.$patch({ user: null, initialized: true });
  router.replace({ name: "login" });
}

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

  // Le jeton est pose tout de suite : `check-access` en a besoin pour
  // s'authentifier. L'IDENTITE, elle, n'est persistee qu'apres son succes.
  setDiscordToken(token);
  utilisateur = user;

  // Nettoie l'URL (retire le fragment sensible) avant la prochaine nav.
  history.replaceState(null, "", window.location.pathname);

  // Verifie que le compte Discord est autorise. L'acces au back-office est
  // reserve aux identifiants listes dans SUPERADMIN_USER_IDS cote API : celle-ci
  // repond 200 si c'est le cas, 403 sinon.
  await verifierLAcces();
});
</script>

<template>
  <div class="callback-page">
    <div class="callback-card">
      <!-- Pas de spinner quand l'attente est terminee et qu'on demande une
           action : une roue qui tourne sous un bouton dit que ca continue. -->
      <div v-if="!reessayable" class="spinner" :class="status"></div>
      <p :class="['message', status]">{{ message }}</p>
      <div v-if="reessayable" class="actions">
        <button type="button" class="btn-retry" @click="verifierLAcces">
          Réessayer
        </button>
        <button type="button" class="btn-back" @click="retourConnexion">
          Retour à la connexion
        </button>
      </div>
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

.actions {
  display: flex;
  gap: 10px;
  justify-content: center;
  flex-wrap: wrap;
  margin-top: 20px;
}
.btn-retry,
.btn-back {
  padding: 8px 16px;
  border-radius: var(--radius-md, 8px);
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
}
.btn-retry {
  border-color: var(--accent);
  background: var(--accent);
  color: #fff;
}
</style>
