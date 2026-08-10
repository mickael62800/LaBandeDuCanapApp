// Acces aux endpoints d'administration d'Atrium.
//
// Miroir exact du contrat expose par `atrium-api/src/admin.rs`. Toute
// evolution se fait des deux cotes en meme temps.

import { atriumGet, atriumPut } from "@/api/atriumHttp";

export interface AtriumState {
  guild_id: string;
  enabled: boolean;
}

/// Consommation du jour et limites REELLEMENT appliquees a ce serveur.
///
/// Les limites viennent de `bot_guild_config` si le serveur en a defini,
/// sinon des variables d'environnement lues au demarrage. Dans les deux cas
/// ce sont les valeurs qui s'appliquent, pas des valeurs theoriques.
export interface AtriumUsage {
  global_used_today: number;
  guild_used_today: number;
  guild_active_users_today: number;
  global_daily_limit: number;
  user_daily_limit: number;
  user_cooldown_secs: number;
}

export interface AtriumDocument {
  id: string;
  title: string;
  source_url: string | null;
  enabled: boolean;
  chunk_count: number;
  updated_at: string;
}

export const atriumService = {
  state: (guildId: string) =>
    atriumGet<AtriumState>(`/admin/guilds/${guildId}/state`),

  setState: (guildId: string, enabled: boolean, actorId: string) =>
    atriumPut<AtriumState>(`/admin/guilds/${guildId}/state`, {
      enabled,
      actor_id: actorId,
    }),

  usage: (guildId: string) =>
    atriumGet<AtriumUsage>(`/admin/guilds/${guildId}/usage`),

  /// Ecrit les cles de config passees. Une cle absente n'est pas touchee :
  /// l'ecran envoie donc uniquement ce qui a change.
  setConfig: (guildId: string, values: Record<string, string>) =>
    atriumPut<{ updated: number }>(`/admin/guilds/${guildId}/config`, { values }),

  knowledge: (guildId: string) =>
    atriumGet<AtriumDocument[]>(`/admin/guilds/${guildId}/knowledge`),
};
