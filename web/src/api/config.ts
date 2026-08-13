// Config persistante (API url + tokens bots) en localStorage pour la version web.
// Le client_id/secret Discord OAuth est gere cote backend, le front n'en
// voit jamais rien.

const K_API = "ds.api.config";
const K_DISCORD_USER = "ds.discord.user";

/// `api_key` a ete RETIRE de ce contrat.
///
/// Le champ datait d'une epoque ou le SPA portait lui-meme un Bearer interne.
/// Il etait vide en production — les secrets Nexus, Ops et Atrium sont injectes
/// par nginx cote serveur — mais la capacite restait : une valeur non vide
/// devenait un `Authorization: Bearer`, lisible par tout JavaScript de
/// l'origine et survivant a la fermeture du navigateur. Une XSS aurait donc eu
/// un secret interne DURABLE a exfiltrer, la ou le jeton Discord vit en
/// `sessionStorage` precisement pour ne pas offrir ca.
///
/// Ne pas le remettre. Un mode developpeur qui en aurait besoin doit etre
/// explicite, limite a localhost, et garder la valeur en memoire.
export interface ApiConfig { api_url: string }
export interface DiscordUser { id: string; username: string; avatar?: string | null; global_name?: string | null; is_superadmin?: boolean }

function parseStored<T>(key: string): T | null {
  const raw = localStorage.getItem(key);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    // Une écriture interrompue ou une ancienne version ne doit pas casser le bootstrap.
    localStorage.removeItem(key);
    return null;
  }
}

/**
 * Origines autorisées pour `api_url`. La config est en localStorage, donc
 * modifiable par tout code s'exécutant dans la page : sans whitelist, une config
 * empoisonnée (`https://evil.com`) détournerait les requêtes ET les tokens
 * qu'elles embarquent (Authorization / X-Discord-Token) vers un serveur tiers.
 *
 * Légitimement, le web pointe toujours vers son propre origin (main.ts pose
 * `window.location.origin`). On autorise donc : l'origin courant, l'éventuel
 * VITE_API_URL fixé au build, et localhost/127.0.0.1 en dev.
 */
function isAllowedApiUrl(value: string): boolean {
  let origin: string;
  try {
    origin = new URL(value).origin;
  } catch {
    return false; // URL malformée
  }
  const allowed = new Set<string>();
  try { allowed.add(window.location.origin); } catch { /* SSR/tests */ }
  const buildUrl = import.meta.env.VITE_API_URL;
  if (buildUrl) {
    try { allowed.add(new URL(buildUrl).origin); } catch { /* ignore */ }
  }
  if (allowed.has(origin)) return true;
  // Dev uniquement : tolère localhost/127.0.0.1 (n'importe quel port).
  if (!import.meta.env.PROD) {
    try {
      const h = new URL(value).hostname;
      if (h === "localhost" || h === "127.0.0.1") return true;
    } catch { /* ignore */ }
  }
  return false;
}

export function getApiConfig(): ApiConfig | null {
  const cfg = parseStored<Partial<ApiConfig> & { api_key?: unknown }>(K_API);
  if (!cfg || typeof cfg.api_url !== "string") {
    if (cfg) localStorage.removeItem(K_API);
    return null;
  }
  // Purge de l'ancien champ sur les navigateurs qui l'ont deja stocke. Sans
  // cette reecriture, retirer `api_key` du code laissait la valeur dormir dans
  // le localStorage des postes existants : plus personne ne l'envoie, mais une
  // XSS peut toujours la lire. La supprimer du contrat ne la supprime pas des
  // machines — il faut y aller.
  if ("api_key" in cfg) {
    delete cfg.api_key;
    localStorage.setItem(K_API, JSON.stringify({ api_url: cfg.api_url }));
  }
  // Assainissement : si l'api_url stockée n'est pas dans la whitelist d'origines,
  // on la ramène à l'origin courant (défaut sûr) au lieu de faire confiance à une
  // valeur potentiellement empoisonnée. Une chaîne vide (= relatif/same-origin)
  // est laissée telle quelle.
  if (cfg && cfg.api_url && !isAllowedApiUrl(cfg.api_url)) {
    cfg.api_url = window.location.origin;
  }
  return cfg as ApiConfig;
}
export function setApiConfig(cfg: ApiConfig) {
  localStorage.setItem(K_API, JSON.stringify(cfg));
}

export function getDiscordUser(): DiscordUser | null {
  const user = parseStored<Partial<DiscordUser>>(K_DISCORD_USER);
  if (!user || typeof user.id !== "string" || typeof user.username !== "string") {
    if (user) localStorage.removeItem(K_DISCORD_USER);
    return null;
  }
  return user as DiscordUser;
}
export function setDiscordUser(u: DiscordUser | null) {
  if (u) localStorage.setItem(K_DISCORD_USER, JSON.stringify(u));
  else localStorage.removeItem(K_DISCORD_USER);
}

// Token Discord OAuth (renseigne apres callback OAuth) envoye en header X-Discord-Token.
//
// SECURITE : stocke en sessionStorage (et non localStorage) pour limiter
// l'exfiltration en cas de XSS persistant. sessionStorage est purge a la
// fermeture du tab/navigateur -> un attaquant doit voler le token "live"
// pendant que le tab est ouvert. Migration douce : on lit aussi l'ancienne
// valeur localStorage pour les sessions existantes, puis on la deplace.
const K_DISCORD_TOKEN = "ds.discord.token";

function migrateFromLocalStorage(): void {
  const legacy = localStorage.getItem(K_DISCORD_TOKEN);
  if (legacy && !sessionStorage.getItem(K_DISCORD_TOKEN)) {
    sessionStorage.setItem(K_DISCORD_TOKEN, legacy);
  }
  if (legacy) {
    localStorage.removeItem(K_DISCORD_TOKEN);
  }
}

export function getDiscordToken(): string {
  migrateFromLocalStorage();
  return sessionStorage.getItem(K_DISCORD_TOKEN) ?? "";
}
export function setDiscordToken(t: string) {
  sessionStorage.setItem(K_DISCORD_TOKEN, t);
  // Au cas ou un ancien token traine en localStorage, on le purge.
  localStorage.removeItem(K_DISCORD_TOKEN);
}
export function clearDiscordToken() {
  sessionStorage.removeItem(K_DISCORD_TOKEN);
  localStorage.removeItem(K_DISCORD_TOKEN);
}
