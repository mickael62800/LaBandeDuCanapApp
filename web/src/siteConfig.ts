// Configuration publique du site, chargée à l'exécution.
//
// Ces valeurs (identifiant du serveur Discord, lien d'invitation) changent
// sans que le code change. Les figer au build via `import.meta.env` obligerait
// à reconstruire l'image pour corriger un lien — d'où un `site-config.json`
// écrit par l'entrypoint nginx et lu une fois au démarrage.
//
// Rien de secret n'y transite : ce fichier est servi à tout visiteur. Il ne
// contient que ce qui figure déjà dans n'importe quelle URL du serveur.

export interface SiteConfig {
  /// Serveur Discord dont on affiche la vie publique.
  guildId: string;
  /// Invitation Discord. Vide = le bouton « Rejoindre » ne s'affiche pas.
  discordInvite: string;
}

import { fetchWithTimeout } from "./api/httpTransport";

const VIDE: SiteConfig = { guildId: "", discordInvite: "" };

let config: SiteConfig = VIDE;

/**
 * Charge la configuration. À appeler une fois avant le montage de l'app.
 *
 * Un échec n'est pas fatal : le site reste consultable, les sections publiques
 * se masquent simplement. Casser toute l'application parce qu'un fichier de
 * configuration manque serait disproportionné.
 */
export async function loadSiteConfig(): Promise<SiteConfig> {
  try {
    // `cache: no-store` : le fichier est réécrit à chaque démarrage du
    // conteneur, un cache navigateur servirait l'ancienne valeur après une
    // correction de configuration.
    const res = await fetchWithTimeout(
      "/site-config.json",
      { cache: "no-store" },
      3_000,
    );
    if (!res.ok) return config;

    const brut = (await res.json()) as { guild_id?: string; discord_invite?: string };
    config = {
      guildId: (brut.guild_id ?? "").trim(),
      // Seuls les liens Discord sont acceptés : cette valeur alimente un
      // `href`, et une URL arbitraire en ferait un vecteur de redirection.
      discordInvite: inviteValide(brut.discord_invite ?? ""),
    };
  } catch {
    // Fichier absent en développement : comportement normal.
  }
  return config;
}

/// Hôtes d'invitation légitimes de Discord.
const HOTES_INVITATION = ["discord.gg", "discord.com", "www.discord.com", "discordapp.com"];

function inviteValide(url: string): string {
  const propre = url.trim();
  if (!propre) return "";
  try {
    const u = new URL(propre);
    if (u.protocol !== "https:") return "";
    return HOTES_INVITATION.includes(u.hostname) ? propre : "";
  } catch {
    return "";
  }
}

/// Configuration courante. Vide tant que `loadSiteConfig` n'a pas abouti.
export function siteConfig(): SiteConfig {
  return config;
}
