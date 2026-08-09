import { nexusGet, nexusPost, nexusDelete } from "@/api/nexusHttp";

export interface MentionableGame {
  id: string;
  guild_id: string;
  game_name: string;
  emoji: string | null;
  category: string | null;
  role_id: string | null;
  subscriber_count: number;
}

export interface CreateMentionableGameDto {
  name: string;
  emoji?: string | null;
  category?: string | null;
  actor_id: string;
}

export interface DeployPanelDto {
  channel_id: string;
  category?: string | null;
}

export const nexusMentionableGamesService = {
  listGames(guildId: string): Promise<MentionableGame[]> {
    return nexusGet<MentionableGame[]>(`/api/games/${encodeURIComponent(guildId)}`, guildId);
  },

  createGame(guildId: string, dto: CreateMentionableGameDto): Promise<MentionableGame> {
    return nexusPost<MentionableGame>(`/api/games/${encodeURIComponent(guildId)}`, guildId, dto);
  },

  deleteGame(guildId: string, gameId: string, actorId: string): Promise<void> {
    return nexusDelete(`/api/games/${encodeURIComponent(guildId)}/${encodeURIComponent(gameId)}?actor_id=${encodeURIComponent(actorId)}`, guildId);
  },

  deployPanel(guildId: string, dto: DeployPanelDto): Promise<{ message_id: string }> {
    return nexusPost<{ message_id: string }>(`/api/games/${encodeURIComponent(guildId)}/panel`, guildId, dto);
  }
};
