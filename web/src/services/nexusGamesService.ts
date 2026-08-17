// Serveurs de jeu de la plateforme Nexus (game-portal).
//
// Passe par la passerelle /nexus-api : l'autorisation est verifiee cote
// serveur par nginx (gate RBAC `nexus.access`) avant d'atteindre nexus-api.

import { nexusDelete, nexusGet, nexusPost, nexusPut } from "@/api/nexusHttp";

/** Etats possibles d'un serveur, tels que renvoyes par l'API. */
export type GameServerStatus =
  | "created"
  | "scheduled"
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "error"
  | "deleted";

export interface GameServer {
  id: string;
  guild_id: string;
  template_id: string;
  name: string;
  status: GameServerStatus;
  host_port: number | null;
  rcon_port: number | null;
  allocated_memory_mb: number;
  cpu_limit: number | null;
  owner_user_id: string;
  last_active_at: string | null;
  last_player_count: number;
  last_error: string | null;
  created_at: string;
  started_at: string | null;
  stopped_at: string | null;
  text_channel_id: string | null;
  voice_channel_id: string | null;
  ip_reveal_at: string | null;
  ip_revealed: boolean;
  /**
   * Hote public, servi a l'administration sans attendre la revelation :
   * celle-ci protege l'adresse des joueurs, pas des administrateurs.
   * `null` = pas encore configure dans le module game-portal.
   */
  public_host: string | null;
}

/**
 * Adresse de connexion complete, ou `null` s'il manque l'hote ou le port.
 * Le port n'est attribue qu'au demarrage du conteneur.
 */
export function adresseServeur(s: GameServer): string | null {
  if (!s.public_host || !s.host_port) return null;
  return `${s.public_host}:${s.host_port}`;
}

/// Un champ configurable d'un template, décrit par son `config_schema`.
/// Le formulaire de création se construit entièrement à partir de ça : ajouter
/// une option à un jeu se fait en base, sans toucher au front.
export interface TemplateField {
  /// Avertissement sur ce que le réglage CASSE, par opposition à
  /// `description` qui explique ce qu'il fait. Affiché distinctement :
  /// noyé dans le texte courant, il ne serait pas lu.
  warning?: string | null;
  key: string;
  label: string;
  /// Section d'affichage. Sans regroupement, cinquante champs a plat
  /// sont inutilisables.
  group?: string | null;
  /// Aide affichee sous le champ.
  description?: string | null;
  type: "text" | "number" | "boolean" | "enum";
  default?: string | number | boolean;
  options?: string[];
  min?: number;
  max?: number;
  max_length?: number;
}

export interface GameTemplate {
  id: string;
  slug: string;
  name: string;
  description?: string | null;
  icon?: string | null;
  category?: string | null;
  accent_color?: string | null;
  cover_image_url?: string | null;
  container_port: number;
  default_memory_mb: number;
  min_memory_mb: number;
  max_memory_mb: number;
  supports_rcon: boolean;
  supports_mods: boolean;
  config_schema: TemplateField[];
}

/** Paramètre d'une commande d'administration. */
export interface CommandParam {
  key: string;
  label: string;
  type: "player" | "text" | "number" | "enum";
  description?: string | null;
  options?: string[];
  min?: number;
  max?: number;
  max_length?: number;
  required?: boolean;
}

/**
 * Une commande proposée par le jeu.
 *
 * Le gabarit RCON n'est volontairement pas exposé : l'écran n'en a pas besoin,
 * et le connaître inviterait à le rejouer à la main.
 */
export interface GameCommand {
  key: string;
  label: string;
  group?: string | null;
  description?: string | null;
  warning?: string | null;
  confirm?: boolean;
  danger?: boolean;
  params?: CommandParam[];
}

/** Un joueur actuellement connecté, tel que le serveur de jeu le rapporte. */
export interface OnlinePlayer {
  name: string;
  /** SteamID64 pour Palworld. C'est lui que prennent les commandes. */
  game_player_id: string | null;
}

export interface GameServerDetail {
  server: GameServer;
  config: Record<string, string>;
}

export interface GameServerStats {
  cpu_percent: number;
  memory_used_mb: number;
  memory_limit_mb: number;
  network_rx_bytes: number;
  network_tx_bytes: number;
}

export interface CreateServerPayload {
  template_slug: string;
  name: string;
  memory_mb?: number;
  cpu_limit?: number;
  owner_user_id: string;
  config: Record<string, string>;
  ip_reveal_days?: number;
}

export interface PlayerSession {
  id: string;
  player_name: string;
  joined_at: string;
  left_at: string | null;
  duration_seconds: number | null;
}

export const nexusGamesService = {
  /** GET /api/games/{guild}/servers */
  listServers(guildId: string): Promise<GameServer[]> {
    return nexusGet<GameServer[]>(`/api/games/${encodeURIComponent(guildId)}/servers`, guildId);
  },

  /** GET /api/games/{guild}/templates — catalogue des jeux disponibles. */
  listTemplates(guildId: string): Promise<GameTemplate[]> {
    return nexusGet<GameTemplate[]>(`/api/games/${encodeURIComponent(guildId)}/templates`, guildId);
  },

  /**
   * POST /api/games/servers/{id}/start
   *
   * L'AUTEUR DE L'ACTION N'EST PLUS ENVOYÉ PAR LE SPA. Il l'était en paramètre
   * d'URL (`?actor_id=`) et l'API le reprenait tel quel : n'importe quelle
   * action tracée — RCON, arrêt, suppression — pouvait donc être attribuée à
   * quelqu'un d'autre. La passerelle nginx pose désormais `X-Actor-Id` depuis
   * la session vérifiée, côté serveur, et l'API ignore ce que le navigateur
   * propose. Ne pas le remettre : ce serait ignoré, et trompeur à la relecture.
   */
  start(guildId: string, serverId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/start`,
      guildId,
    );
  },

  /** POST /api/games/servers/{id}/stop */
  stop(guildId: string, serverId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/stop`,
      guildId,
    );
  },

  /** POST /api/games/servers/{id}/restart */
  restart(guildId: string, serverId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/restart`,
      guildId,
    );
  },

  /** POST /api/games/servers/{id}/reveal-ip — révélation anticipée admin. */
  revealIp(guildId: string, serverId: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/reveal-ip`,
      guildId,
    );
  },

  /**
   * POST /api/games/servers/{id}/schedule — mode « Préparation ».
   * Programme l'ouverture sans démarrer le conteneur : le serveur passe
   * `scheduled`, les salons/panneau sont créés tout de suite, et le worker
   * démarre le conteneur ~5 min avant `revealAt` (ISO 8601).
   */
  schedule(guildId: string, serverId: string, revealAt: string): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/schedule`,
      guildId,
      { reveal_at: revealAt },
    );
  },

  /**
   * POST /api/games/servers/{id}/reveal-schedule — programme (ou efface avec
   * `null`) l'heure de révélation auto de l'IP sans changer l'état du conteneur.
   */
  setRevealSchedule(
    guildId: string,
    serverId: string,
    revealAt: string | null,
  ): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/reveal-schedule`,
      guildId,
      { reveal_at: revealAt },
    );
  },

  /** GET /api/games/servers/{id} — detail + configuration effective. */
  getServer(guildId: string, serverId: string): Promise<GameServerDetail> {
    return nexusGet<GameServerDetail>(
      `/api/games/servers/${encodeURIComponent(serverId)}`,
      guildId,
    );
  },

  /** POST /api/games/{guild}/servers — cree un serveur. */
  create(guildId: string, payload: CreateServerPayload): Promise<GameServer> {
    return nexusPost<GameServer>(
      `/api/games/${encodeURIComponent(guildId)}/servers`,
      guildId,
      payload,
    );
  },

  /** GET /api/games/servers/{id}/logs — dernieres lignes du conteneur. */
  logs(guildId: string, serverId: string, lines = 200): Promise<string[]> {
    return nexusGet<string[]>(
      `/api/games/servers/${encodeURIComponent(serverId)}/logs?lines=${lines}`,
      guildId,
    );
  },

  /** GET /api/games/servers/{id}/stats — CPU / RAM / reseau en direct. */
  stats(guildId: string, serverId: string): Promise<GameServerStats> {
    return nexusGet<GameServerStats>(
      `/api/games/servers/${encodeURIComponent(serverId)}/stats`,
      guildId,
    );
  },

  /** PUT /api/games/servers/{id}/config — enregistre les overrides. */
  updateConfig(
    guildId: string,
    serverId: string,
    config: Record<string, string>,
  ): Promise<void> {
    return nexusPut<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/config`,
      guildId,
      { config },
    );
  },

  /** POST /api/games/servers/{id}/command — commande RCON. */
  rcon(guildId: string, serverId: string, command: string): Promise<{ response: string }> {
    return nexusPost<{ response: string }>(
      `/api/games/servers/${encodeURIComponent(serverId)}/command`,
      guildId,
      { command },
    );
  },

  /** GET /api/games/servers/{id}/commands — catalogue d'administration du jeu. */
  commands(guildId: string, serverId: string): Promise<GameCommand[]> {
    return nexusGet<GameCommand[]>(
      `/api/games/servers/${encodeURIComponent(serverId)}/commands`,
      guildId,
    );
  },

  /**
   * POST /api/games/servers/{id}/commands/{key} — exécute une commande du
   * catalogue. On envoie une clé et des paramètres, jamais une commande :
   * c'est l'API qui compose, à partir du gabarit qu'elle seule connaît.
   */
  runCommand(
    guildId: string,
    serverId: string,
    commandKey: string,
    params: Record<string, string>,
  ): Promise<{ response: string }> {
    return nexusPost<{ response: string }>(
      `/api/games/servers/${encodeURIComponent(serverId)}/commands/${encodeURIComponent(commandKey)}`,
      guildId,
      { params },
    );
  },

  /** GET /api/games/servers/{id}/players/online — interroge le jeu en direct. */
  onlinePlayers(guildId: string, serverId: string): Promise<OnlinePlayer[]> {
    return nexusGet<OnlinePlayer[]>(
      `/api/games/servers/${encodeURIComponent(serverId)}/players/online`,
      guildId,
    );
  },

  /** GET /api/games/servers/{id}/sessions — historique des joueurs. */
  sessions(guildId: string, serverId: string): Promise<PlayerSession[]> {
    return nexusGet<PlayerSession[]>(
      `/api/games/servers/${encodeURIComponent(serverId)}/sessions`,
      guildId,
    );
  },

  /** DELETE /api/games/servers/{id} */
  remove(guildId: string, serverId: string): Promise<void> {
    return nexusDelete<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}`,
      guildId,
    );
  },
};
