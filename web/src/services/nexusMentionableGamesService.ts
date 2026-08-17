import { nexusGet, nexusPost, nexusDelete } from "@/api/nexusHttp";

export interface MentionableGame {
  id: string;
  guild_id: string;
  game_name: string;
  emoji: string | null;
  category: string | null;
  role_id: string | null;
}

export interface CreateMentionableGameDto {
  guild_id: string;
  game_name: string;
  emoji?: string | null;
  category?: string | null;
  created_by: string;
}

export interface DeployPanelDto {
  channel_id: string;
  category?: string | null;
}

/**
 * Un écart constaté entre la base et Discord.
 *
 * `kind` décide des champs présents — l'API sérialise l'énumération du domaine
 * à plat. Aucune réparation n'est déduite ici : c'est l'API qui tranche, et
 * seulement sur demande explicite.
 */
export type SyncDivergence =
  | { kind: "role_missing"; key: string; game_id: string; game_name: string; role_id: string }
  | { kind: "role_unbound"; key: string; game_id: string; game_name: string }
  | { kind: "role_orphan"; key: string; role_id: string; role_name: string }
  | {
      kind: "panel_message_missing";
      key: string;
      panel_id: string;
      channel_id: string;
      message_id: string;
    };

export interface SyncReport {
  /** `null` = le bot n'a jamais rendu compte : état inconnu, pas « tout va bien ». */
  inventory_taken_at: string | null;
  divergences: SyncDivergence[];
}

/** Quel côté fait foi pour résoudre un écart. */
export type SyncDirection = "discord" | "dashboard";

export interface SyncResolution {
  key: string;
  applied_now: boolean;
  requested_from_discord: boolean;
  detail: string;
}

export const nexusMentionableGamesService = {
  listGames(guildId: string): Promise<MentionableGame[]> {
    return nexusGet<MentionableGame[]>(`/api/games/${encodeURIComponent(guildId)}`, guildId);
  },

  createGame(guildId: string, dto: CreateMentionableGameDto): Promise<MentionableGame> {
    return nexusPost<MentionableGame>(`/api/games`, guildId, dto);
  },

  deleteGame(guildId: string, gameId: string, actorId: string): Promise<void> {
    return nexusDelete(`/api/games/${encodeURIComponent(guildId)}/${encodeURIComponent(gameId)}?actor_id=${encodeURIComponent(actorId)}`, guildId);
  },

  deployPanel(guildId: string, dto: DeployPanelDto): Promise<void> {
    return nexusPost<void>(`/api/games/${encodeURIComponent(guildId)}/panel/deploy`, guildId, dto);
  },

  /** Rapport de divergence, calculé sur la dernière photographie connue. */
  getSyncReport(guildId: string): Promise<SyncReport> {
    return nexusGet<SyncReport>(`/api/games/${encodeURIComponent(guildId)}/sync`, guildId);
  },

  /**
   * Demande une photographie fraîche au bot. Répond 202 : la vérification est
   * lancée, pas terminée — le rapport ne change qu'une fois le bot passé.
   */
  requestSyncCheck(guildId: string): Promise<void> {
    return nexusPost<void>(`/api/games/${encodeURIComponent(guildId)}/sync/check`, guildId);
  },

  /** Résout un écart dans la direction choisie. */
  resolveSync(
    guildId: string,
    key: string,
    direction: SyncDirection,
  ): Promise<SyncResolution> {
    return nexusPost<SyncResolution>(
      `/api/games/${encodeURIComponent(guildId)}/sync/resolve`,
      guildId,
      { key, direction },
    );
  },
};
