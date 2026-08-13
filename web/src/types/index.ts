export interface Guild {
  guild_id: string;
  name: string;
  icon: string | null;
  member_count: number;
}

export interface BotDefinition {
  bot_name: string;
  display_name: string;
  description: string;
  config_schema: ConfigField[];
}

export interface ConfigFieldOption {
  value: string;
  label: string;
}

export interface ConfigField {
  key: string;
  label: string;
  /**
   * Type d'input :
   * - "boolean" : toggle ON/OFF
   * - "number" : input numerique (peut etre combine avec unit / min / max)
   * - "channel" : dropdown des salons Discord
   * - "role"    : dropdown des roles Discord
   * - "enum"    : <select> base sur le tableau `options`
   * - "text"    : input texte libre
   */
  type: string;
  required: boolean;
  default?: string;
  /**
   * Description courte (1-2 phrases) affichee a droite de l'input.
   * Aide pedagogique pour expliquer ce que fait le champ.
   */
  description?: string;
  /**
   * Unite affichee en suffixe d'un input number (ex: "heures", "minutes",
   * "secondes", "%"). Cosmetique — n'affecte pas la valeur stockee.
   */
  unit?: string;
  /** Valeur min autorisee pour un input number (clamp dur a la sauvegarde). */
  min?: number;
  /** Valeur max autorisee pour un input number (clamp dur a la sauvegarde). */
  max?: number;
  /** Options pour type="enum" (dropdown). */
  options?: ConfigFieldOption[];
  /**
   * Si true, ce reglage n'est lu qu'au demarrage du composant : une
   * modification ne prend effet qu'apres un redemarrage. On affiche un
   * badge d'avertissement a cote du label. Optionnel (retro-compat).
   */
  restart_required?: boolean;
  [k: string]: unknown;
}

export interface DiscordChannelInfo {
  id: string;
  name: string;
  position?: number;
  /** "text" | "announcement" | "voice" | "category" | "stage" — defaut
   *  "text" pour retro-compat avec les anciens endpoints. */
  kind?: "text" | "announcement" | "voice" | "category" | "stage";
}

export interface BotGuildConfig {
  guild_id: string;
  bot_name: string;
  config_key: string;
  config_value: string;
}

/// Miroir de `api/config.ts` : `api_key` y a ete retire, ne pas le
/// reintroduire ici (le SPA ne porte plus aucun secret de service).
export interface ApiConfig {
  api_url: string;
}

export interface DiscordUser {
  id: string;
  username: string;
  discriminator: string;
  avatar: string | null;
  global_name: string | null;
}

export interface ServerStats {
  total_servers: number;
  total_users: number;
  messages_today: number;
  infractions_today: number;
  bots_online: number;
  bots_total: number;
  workers_online: number;
  workers_total: number;
  postgres_online: boolean;
  redis_online: boolean;
}

export interface LogEntry {
  id: string;
  timestamp: string;
  level: string;
  bot: string;
  server: string;
  message: string;
  category: string;
  details: Record<string, unknown>;
}

export interface Infraction {
  id: string;
  user_id: string;
  username: string;
  /** Pseudo serveur (nickname) si configure, sinon null. */
  display_name?: string | null;
  server: string;
  infraction_type: string;
  action?: string;
  reason: string;
  score?: number;
  created_at: string;
  moderator: string;
  /**
   * "detection" = proposition automod (table `infractions`).
   * "action"    = sanction appliquee (table `moderation_actions`).
   */
  source?: "detection" | "action";
  /** Duree en secondes (mute/timeout/ban temporaire). */
  duration?: number;
  /** Contenu original du message analyse (uniquement source="detection"). */
  content?: string;
}

export interface ConfirmedBan {
  id: string;
  guild_id: string;
  target_id: string;
  target_name: string;
  /** Pseudo serveur (nickname) actuel, si l'user etait dans la guild. */
  target_display_name?: string | null;
  moderator_name: string;
  action_type: string;
  reason: string;
  created_at: string;
}

export interface ModerationRule {
  id: string;
  name: string;
  enabled: boolean;
  rule_type: string;
  action: string;
  description: string;
  /// Valeurs réelles enregistrées, servies par l'API.
  weight: number;
  threshold_warn: number;
  threshold_delete: number;
  threshold_mute: number;
  threshold_ban: number;
}

export interface UpdateRuleParams {
  guild_id: string;
  flag_type: string;
  weight: number;
  threshold_warn: number;
  threshold_delete: number;
  threshold_mute: number;
  threshold_ban: number;
  enabled: boolean;
}

export interface TableColumn {
  key: string;
  label: string;
}

export interface Notification {
  id: string;
  notification_type: string;
  title: string;
  message: string;
  severity: string;
  read: boolean;
  created_at: string;
}

export interface SecurityEvent {
  id: string;
  guild_id: string;
  event_type: string;
  severity: string;
  description: string;
  user_ids: string[];
  created_at: string;
}

export interface ModerationActionResponse {
  id: string;
  action_type: string;
  target_name: string;
  reason: string;
}

export interface UserModerationHistory {
  target_id: string;
  target_name: string;
  total_warns: number;
  total_mutes: number;
  total_bans: number;
  actions: ModerationActionResponse[];
}

export interface SelectOption {
  value: string;
  label: string;
}

export interface UserActivity {
  id: string;
  guild_id: string;
  user_id: string;
  event_type: string;
  channel_id: string | null;
  channel_name: string | null;
  content: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface GuildMember {
  id: string;
  username: string;
  display_name: string | null;
  avatar_url: string | null;
}

export interface Ticket {
  id: string;
  title: string;
  status: string;
  priority: string;
  author_id: string;
  author_name: string;
  assigned_to: string | null;
  server: string;
  category: string;
  ticket_type: string | null;
  channel_id: string | null;
  voice_channel_id: string | null;
  invited_user_id: string | null;
  created_at: string;
  updated_at: string;
  messages_count: number;
}

export interface TicketMessage {
  id: string;
  ticket_id: string;
  author_name: string;
  author_role: string;
  content: string;
  created_at: string;
}

export interface TicketDetail {
  ticket: Ticket;
  messages: TicketMessage[];
}

// ── Voice Channels ──

export interface VoiceChannel {
  id: string;
  guild_id: string;
  owner_id: string;
  owner_name: string;
  channel_id: string;
  text_channel_id: string | null;
  members_channel_id: string | null;
  queue_channel_id: string | null;
  category_id: string | null;
  channel_name: string;
  kind: string;
  visibility: string;
  queue_enabled: boolean;
  locked: boolean;
  member_limit: number | null;
  status: string | null;
  created_at: string;
}

export interface VoiceChannelCoAdmin {
  id: string;
  user_id: string;
  user_name: string;
  granted_at: string;
}

export interface VoiceChannelBan {
  id: string;
  user_id: string;
  user_name: string;
  banned_by: string;
  reason: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface VoiceChannelDetail {
  channel: VoiceChannel;
  co_admins: VoiceChannelCoAdmin[];
  bans: VoiceChannelBan[];
}

// ── Role Panels ──

export interface RolePanel {
  id: string;
  guild_id: string;
  channel_id: string;
  message_id: string | null;
  title: string;
  description: string;
  mode: string;
  max_roles: number | null;
  enabled: boolean;
  created_at: string;
}

export interface RolePanelEntry {
  id: string;
  role_id: string;
  role_name: string;
  emoji: string | null;
  label: string;
  style: string;
  position: number;
}

export interface RolePanelDetail {
  panel: RolePanel;
  entries: RolePanelEntry[];
}

export interface AutoRoleConfig {
  id: string;
  guild_id: string;
  role_id: string;
  role_name: string;
  delay_secs: number;
  enabled: boolean;
}

// ── Dashboard Charts ──

export interface DailyActivity {
  day: string;
  messages: number;
  voice_minutes: number;
  active_members: number;
  new_members: number;
  leaves: number;
  infractions: number;
  warns: number;
  mutes: number;
  bans: number;
}

export interface TopUser {
  user_id: string;
  username: string;
  message_count: number;
  voice_seconds: number;
  voice_hours: number;
}

// ── Levels / XP ──

export interface UserLevel {
  id: string;
  guild_id: string;
  user_id: string;
  username: string;
  xp: number;
  level: number;
  xp_current: number;
  xp_needed: number;
  xp_text: number;
  level_text: number;
  xp_text_current: number;
  xp_text_needed: number;
  xp_voice: number;
  level_voice: number;
  xp_voice_current: number;
  xp_voice_needed: number;
  last_xp_at: string;
}

// ── Audit Logs ──

export interface AuditLog {
  id: string;
  guild_id: string;
  event_type: string;
  actor_id: string | null;
  actor_name: string | null;
  target_id: string | null;
  target_name: string | null;
  channel_id: string | null;
  channel_name: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

// ── Watched Users (Surveillance) ──

export interface WatchedUser {
  user_id: string;
  username: string;
  guild_id: string;
  guild_name: string;
  risk_level: string;
  total_warns: number;
  total_mutes: number;
  total_bans: number;
  last_incident_at: string | null;
  security_events_count: number;
  first_seen_at: string;
}

export interface DossierNote {
  author_name: string;
  content: string;
  created_at?: string;
}

export interface UserDossier {
  user: WatchedUser;
  infractions: Infraction[];
  moderation_actions: ModerationActionResponse[];
  security_events: SecurityEvent[];
  notes?: DossierNote[];
}

// ── Analytics ──

export interface HeatmapPoint {
  hour: number;
  day_of_week: number;
  day_name: string;
  messages: number;
  infractions: number;
}

export interface ActionDistribution {
  action: string;
  count: number;
  percentage: number;
}

export interface TopInfractor {
  user_id: string;
  username: string;
  total_infractions: number;
  warns: number;
  deletes: number;
  mutes: number;
  bans: number;
}

export interface ModerationTrend {
  day: string;
  total: number;
  warns: number;
  deletes: number;
  mutes: number;
  bans: number;
}

export interface PeakHour {
  hour: number;
  label: string;
  avg_messages: number;
  avg_infractions: number;
}

export interface FullAnalytics {
  heatmap: HeatmapPoint[];
  action_distribution: ActionDistribution[];
  top_infractors: TopInfractor[];
  moderation_trend: ModerationTrend[];
  peak_hours: PeakHour[];
}

// Discord Roles (synchronises par le community-bot)
export interface DiscordRole {
  id: string;
  guild_id: string;
  name: string;
  color: number;
  position: number;
  permissions: string;
  mentionable: boolean;
  managed: boolean;
  icon: string | null;
  member_count: number;
  synced_at: string;
}

// ── Members (page Membres) ──

export interface Member {
  guild_id: string;
  user_id: string;
  username: string;
  display_name: string | null;
  avatar: string | null;
  roles: string[];
  joined_at: string | null;
  account_created: string | null;
  is_bot: boolean;
  last_seen_at: string | null;
  /** Set quand le membre a quitte le serveur. NULL = encore actif. */
  left_at?: string | null;
}

/**
 * Une infraction recente telle que serialisee par le backend
 * (`get_member_summary` -> infractions.recent). Cf.
 * sentinel-core manage_members_service.rs.
 */
export interface MemberInfractionRecent {
  id: string;
  created_at: string;
  reason: string;
  /** Score automod (null si non applicable). */
  score: number | null;
  /** Libelle d'action ("warn", "ban", "détection: spam"...). */
  action: string;
  /** Contenu original du message analyse (null si indisponible). */
  content: string | null;
}

/**
 * Une action de moderation recente (`get_member_summary` -> moderation.actions).
 */
export interface MemberModerationAction {
  action_type: string;
  reason: string;
  moderator_name: string;
  created_at: string;
  /** Duree en secondes (mute/ban temporaire), null sinon. */
  duration: number | null;
}

export interface MemberInfractions {
  total: number;
  recent: MemberInfractionRecent[];
}

export interface MemberModeration {
  total_warns: number;
  total_mutes: number;
  total_bans: number;
  actions: MemberModerationAction[];
}

export interface MemberStats {
  message_count: number;
  voice_seconds: number;
  last_active: string | null;
}

export interface MemberSummary {
  member: Member;
  infractions: MemberInfractions;
  moderation: MemberModeration;
  stats: MemberStats;
}

// ── Guild Backup (snapshots serveur : roles + salons) ──

export interface SnapshotSummary {
  id: string;
  label: string;
  created_at: string;
  role_count: number;
  channel_count: number;
}
