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
  /** Noms libres des salons. `null` = suivre le modèle de la guilde. */
  /** Règlement de la soirée, affiché mot pour mot sous l'annonce d'Atrium. */
  rules: string | null;
  channel_name_registration: string | null;
  channel_name_private: string | null;
  channel_name_voice: string | null;
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

/**
 * Seuils de supervision d'un serveur.
 *
 * L'URL du webhook n'y figure pas : c'est un secret côté serveur. L'écran sait
 * seulement qu'un webhook est configuré.
 */
export interface AlertSettings {
  configured: boolean;
  cpu_threshold: number;
  ram_threshold: number;
  latency_threshold_ms: number;
}

/** Une plage d'ouverture, en minutes depuis minuit, valable certains jours. */
export interface TimeRange {
  start_minute: number;
  end_minute: number;
  /**
   * Jours d'application, en bits : lundi = 1, mardi = 2, … dimanche = 64.
   *
   * Facultatif à la lecture : les plages enregistrées avant l'existence des
   * jours n'ont pas le champ, et valent alors toute la semaine. Le composable
   * comble ce trou plutôt que de laisser `undefined` circuler.
   */
  days?: number;
}

/**
 * Plages d'ouverture d'un serveur.
 *
 * Les heures sont LOCALES, exprimées dans `timezone` : un décalage figé
 * ouvrirait le serveur avec une heure d'écart la moitié de l'année.
 */
export type ScheduleMode = "ranges" | "restart";

export interface ServerSchedule {
  enabled: boolean;
  /**
   * Lequel des deux systèmes pilote ce serveur. Ils s'excluent : des plages
   * éteignent le serveur la nuit, une permanence le rallume.
   */
  mode: ScheduleMode;
  timezone: string;
  ranges: TimeRange[];
  warn_minutes: number;
  /** Prochaine ouverture calculée par le serveur. */
  next_opening: string | null;
  /** Réglages de redémarrage automatique du jeu neutralisés par les plages. */
  disabled_restart_keys: string[];
  /** Mode permanence : heures entre deux redémarrages. */
  restart_interval_hours: number | null;
  restart_anchor_minute: number;
  /** Prochain redémarrage calculé par le serveur. */
  next_restart: string | null;
  /**
   * Cadences proposées. Envoyées par le serveur pour que la liste affichée ne
   * puisse pas diverger de ce que l'API accepte.
   */
  restart_interval_choices: number[];
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
  /** Débit dérivé du contrôle de santé précédent ; `null` si pas comparable. */
  network_rx_bytes_per_sec: number | null;
  network_tx_bytes_per_sec: number | null;
  /**
   * Temps de réponse du jeu à la dernière commande de contrôle.
   *
   * Le signal de lag le plus direct : CPU et RAM disent ce que le conteneur
   * consomme, celui-ci dit ce que le serveur met à répondre.
   */
  rcon_latency_ms: number | null;
}

export interface CreateServerPayload {
  template_slug: string;
  name: string;
  memory_mb?: number;
  cpu_limit?: number;
  owner_user_id: string;
  config: Record<string, string>;
  ip_reveal_days?: number;
  /** Règlement de la soirée. Affiché mot pour mot sous l'annonce. */
  rules?: string | null;
}

export interface PlayerSession {
  id: string;
  player_name: string;
  joined_at: string;
  left_at: string | null;
  duration_seconds: number | null;
}

/// Un point de surveillance, deja resume par la base.
///
/// Chaque mesure reste nullable jusqu'a l'affichage : sur une tranche ou la
/// console etait muette, la latence n'est pas nulle, elle est inconnue — et une
/// courbe qui retombe a zero raconte une panne qui n'a pas eu lieu.
export interface PointDeSurveillance {
  horodatage: string;
  cpu_percent: number | null;
  memory_used_mb: number | null;
  memory_limit_mb: number | null;
  rcon_latency_ms: number | null;
  net_rx_bytes_per_sec: number | null;
  net_tx_bytes_per_sec: number | null;
  player_count: number | null;
}

export interface HistoriqueSurveillance {
  points: PointDeSurveillance[];
  /// Plage et pas REELLEMENT appliques : l'API elargit le pas quand la demande
  /// produirait trop de points. L'ecran les affiche, sans quoi une courbe
  /// degrossie passerait pour une perte de mesures.
  range_secs: number;
  step_secs: number;
}

/// Une page d'historique, et de quoi savoir ce qu'il y a derriere.
export interface PageDeSessions {
  items: PlayerSession[];
  /// Nombre total de sessions du serveur, toutes pages confondues. Sans lui,
  /// l'ecran ne peut annoncer ni son nombre de pages ni son total.
  total: number;
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
  /**
   * Programme l'ouverture, et l'heure de fin quand elle est connue.
   *
   * Sans heure de fin, un conteneur arrêté ne peut pas être distingué d'une
   * session terminée : la carte annoncerait « fermé » au milieu d'une soirée.
   */
  schedule(
    guildId: string,
    serverId: string,
    revealAt: string,
    closesAt?: string | null,
  ): Promise<void> {
    return nexusPost<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/schedule`,
      guildId,
      { reveal_at: revealAt, closes_at: closesAt ?? null },
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

  /**
   * PUT /api/games/servers/{id}/resources — ajuste mémoire et cœurs.
   *
   * Docker fige ces limites à la création du conteneur : le changement prend
   * effet au prochain démarrage, qui le reconstruit.
   */
  updateResources(
    guildId: string,
    serverId: string,
    memoryMb: number,
    cpuLimit: number | null,
  ): Promise<void> {
    return nexusPut<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/resources`,
      guildId,
      { memory_mb: memoryMb, cpu_limit: cpuLimit },
    );
  },

  /**
   * PUT /api/games/servers/{id}/channel-names — noms libres des salons.
   *
   * Les trois voyagent ensemble : un champ vidé signifie « reviens au modèle
   * de la guilde », pas « ne change rien ». L'API renomme aussi les salons
   * déjà créés.
   */
  updateChannelNames(
    guildId: string,
    serverId: string,
    noms: {
      channel_name_registration: string | null;
      channel_name_private: string | null;
      channel_name_voice: string | null;
    },
  ): Promise<void> {
    return nexusPut<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/channel-names`,
      guildId,
      noms,
    );
  },

  /**
   * PUT /api/games/servers/{id}/rules — règlement de la soirée.
   *
   * Modifier le texte vaut pour la PROCHAINE annonce : celle déjà publiée
   * garde le règlement en vigueur ce jour-là, ce qui est la bonne lecture d'un
   * règlement.
   */
  updateRules(guildId: string, serverId: string, rules: string | null): Promise<void> {
    return nexusPut<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/rules`,
      guildId,
      { rules },
    );
  },

  /**
   * POST /api/games/servers/{id}/backup — sauvegarde du monde à la demande.
   *
   * Ni l'interrupteur de configuration ni le délai minimal ne s'appliquent :
   * ce sont des garde-fous contre l'archivage automatique répétitif, pas
   * contre un geste délibéré.
   *
   * `a_chaud` vaut vrai quand le serveur tournait : la copie peut alors
   * contenir un fichier à moitié écrit, et l'écran doit le dire.
   */
  backupNow(
    guildId: string,
    serverId: string,
  ): Promise<{ size_bytes: number; a_chaud: boolean }> {
    return nexusPost<{ size_bytes: number; a_chaud: boolean }>(
      `/api/games/servers/${encodeURIComponent(serverId)}/backup`,
      guildId,
      {},
    );
  },

  /** GET /api/games/servers/{id}/schedule-ranges — plages d'ouverture. */
  getScheduleRanges(guildId: string, serverId: string): Promise<ServerSchedule> {
    return nexusGet<ServerSchedule>(
      `/api/games/servers/${encodeURIComponent(serverId)}/schedule-ranges`,
      guildId,
    );
  },

  /** PUT /api/games/servers/{id}/schedule-ranges */
  saveScheduleRanges(
    guildId: string,
    serverId: string,
    schedule: {
      enabled: boolean;
      mode: ScheduleMode;
      timezone: string;
      ranges: TimeRange[];
      warn_minutes: number;
      restart_interval_hours: number | null;
      restart_anchor_minute: number;
    },
  ): Promise<ServerSchedule> {
    return nexusPut<ServerSchedule>(
      `/api/games/servers/${encodeURIComponent(serverId)}/schedule-ranges`,
      guildId,
      schedule,
    );
  },

  /** GET /api/games/servers/{id}/alerts — seuils de supervision. */
  getAlertSettings(guildId: string, serverId: string): Promise<AlertSettings> {
    return nexusGet<AlertSettings>(
      `/api/games/servers/${encodeURIComponent(serverId)}/alerts`,
      guildId,
    );
  },

  /**
   * PUT /api/games/servers/{id}/alerts — enregistre les seuils.
   *
   * `webhookUrl` vide conserve celui déjà enregistré : l'écran ne le connaît
   * pas — c'est un secret, il ne repart jamais du serveur — donc il ne peut
   * pas le renvoyer à chaque modification de seuil.
   */
  saveAlertSettings(
    guildId: string,
    serverId: string,
    settings: {
      webhook_url?: string;
      cpu_threshold: number;
      ram_threshold: number;
      latency_threshold_ms: number;
    },
  ): Promise<void> {
    return nexusPut<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/alerts`,
      guildId,
      settings,
    );
  },

  /** DELETE /api/games/servers/{id}/alerts — arrête la surveillance. */
  deleteAlertSettings(guildId: string, serverId: string): Promise<void> {
    return nexusDelete<void>(
      `/api/games/servers/${encodeURIComponent(serverId)}/alerts`,
      guildId,
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
  sessions(
    guildId: string,
    serverId: string,
    options: { limit?: number; offset?: number } = {},
  ): Promise<PageDeSessions> {
    // L'historique n'est jamais purge : sans bornes, l'ecran demandait tout et
    // n'en affichait qu'un debut arbitraire. On demande donc une page, et le
    // total qui l'accompagne dit ce qu'il reste derriere.
    const parametres = new URLSearchParams();
    if (options.limit !== undefined) parametres.set("limit", String(options.limit));
    if (options.offset !== undefined) parametres.set("offset", String(options.offset));
    const requete = parametres.toString();
    return nexusGet<PageDeSessions>(
      `/api/games/servers/${encodeURIComponent(serverId)}/sessions${requete ? `?${requete}` : ""}`,
      guildId,
    );
  },

  /** GET /api/games/servers/{id}/perf-history — surveillance sur une plage. */
  perfHistory(
    guildId: string,
    serverId: string,
    rangeSecs: number,
    stepSecs?: number,
  ): Promise<HistoriqueSurveillance> {
    const parametres = new URLSearchParams({ range_secs: String(rangeSecs) });
    if (stepSecs !== undefined) parametres.set("step_secs", String(stepSecs));
    return nexusGet<HistoriqueSurveillance>(
      `/api/games/servers/${encodeURIComponent(serverId)}/perf-history?${parametres}`,
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
