// Site communautaire public — appels SANS authentification.
//
// Volontairement separe de `api/http.ts` : ce dernier envoie le token Discord,
// tente un refresh de session sur 401 et redirige vers /login. Sur une page
// publique, ce comportement expulserait un visiteur non connecte hors du site.
// Ici, aucune credential n'est envoyee et une erreur reste une erreur.

import { publicGet } from "./publicHttp";

export interface PublicGuild {
  guild_id: string;
  name: string;
  icon: string | null;
  member_count: number;
}

export const publicSiteService = {
  /** GET /api/public/guilds/{id} — vitrine du serveur. */
  guild(guildId: string): Promise<PublicGuild> {
    return publicGet<PublicGuild>(`/guilds/${encodeURIComponent(guildId)}`);
  },
};

/// URL de l'icone Discord d'un serveur, ou null si aucune.
export function guildIconUrl(g: PublicGuild): string | null {
  return g.icon
    ? `https://cdn.discordapp.com/icons/${g.guild_id}/${g.icon}.png?size=128`
    : null;
}
