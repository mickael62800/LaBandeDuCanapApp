use platform_common_worker::{SECS_PER_HOUR, SECS_PER_MINUTE};

// ── Defauts cleanup ──
pub(super) const DEFAULT_VOICE_SESSIONS_RETENTION_DAYS: i64 = 90;
pub(super) const DEFAULT_LOGS_RETENTION_DAYS: i64 = 30;
pub(super) const DEFAULT_CLOSED_TICKETS_RETENTION_DAYS: i64 = 180;
pub(super) const DEFAULT_CLEANUP_INTERVAL_HOURS: u64 = 1;
pub(super) const DEFAULT_VACUUM_INTERVAL_HOURS: u64 = 24;

// ── Defauts cache (warm Redis) ──
pub(super) const DEFAULT_ANALYTICS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
pub(super) const DEFAULT_DASHBOARD_REFRESH_SECS: u64 = 10 * SECS_PER_MINUTE;
pub(super) const DEFAULT_VOICE_STATS_REFRESH_SECS: u64 = SECS_PER_HOUR;
pub(super) const DEFAULT_LEADERBOARDS_REFRESH_SECS: u64 = 5 * SECS_PER_MINUTE;
pub(super) const DEFAULT_USER_CACHE_SYNC_SECS: u64 = 15 * SECS_PER_MINUTE;
pub(super) const DEFAULT_PARTITION_MANAGER_SECS: u64 = 24 * SECS_PER_HOUR;

// ── Defauts audit_cache ──
pub(super) const DEFAULT_AUDIT_CACHE_REFRESH_SECS: u64 = 60;
/// Limite du snapshot watched_users pousse en Redis (global, non per-guild).
pub(super) const DEFAULT_WATCHED_USERS_QUERY_LIMIT: i64 = 10_000;

// ── Defauts monitoring ──
pub(super) const DEFAULT_MONITOR_CHECK_INTERVAL_SECS: u64 = 30;

// ── Defauts analytics ──
pub(super) const DEFAULT_DAILY_SNAPSHOT_HOURS: u64 = 1;
pub(super) const DEFAULT_HOURLY_SNAPSHOT_MINUTES: u64 = 60;
/// Retention announcement_runs : 1x/jour.
pub(super) const DEFAULT_ANNOUNCEMENTS_RETENTION_SECS: u64 = 24 * SECS_PER_HOUR;
/// Retention cleanup tick : 1x/jour est largement suffisant pour purger
/// les snapshots > data_retention_days (cote API).
pub(super) const DEFAULT_RETENTION_CLEANUP_SECS: u64 = 24 * SECS_PER_HOUR;
/// Top users publish tick : 1x/heure. L'API ne publie reellement que si
/// `top_users_publish_interval_days` est ecoule depuis le dernier post.
pub(super) const DEFAULT_TOP_USERS_PUBLISH_CHECK_SECS: u64 = SECS_PER_HOUR;

// ── Defauts temp_roles ──
pub(super) const DEFAULT_TEMP_ROLES_SCAN_SECS: u64 = SECS_PER_MINUTE;

// ── Defauts appeal_sla ──
pub(super) const DEFAULT_APPEAL_SLA_SCAN_SECS: u64 = 120;

// ── Defauts export ──
pub(super) const DEFAULT_EXPORT_SCAN_SECS: u64 = 5;
/// Nombre max de lignes par export (garde-fou memoire).
pub(super) const DEFAULT_MAX_ROWS_PER_EXPORT: i64 = 50_000;
/// Timeout au-dela duquel un export 'processing' est considere zombie.
pub(super) const DEFAULT_EXPORT_PROCESSING_TIMEOUT_SECS: i64 = 300;

// ── Defauts discord_audit_sync ──
pub(super) const DEFAULT_AUDIT_SYNC_SECS: u64 = 300;

// ── Defauts ai ──
pub(super) const DEFAULT_AI_POLL_SECS: u64 = 2;
pub(super) const DEFAULT_AI_JOB_TIMEOUT_SECS: u64 = 2 * SECS_PER_MINUTE;
/// Taille du batch de jobs IA claimes par tick (garde 1..100).
pub(super) const DEFAULT_AI_BATCH_SIZE: i32 = 5;

// ── Defauts announcements ──
/// Cadence de publication des annonces dues (alignee HH:00 par defaut).
pub(super) const DEFAULT_ANNOUNCEMENT_PUBLISH_INTERVAL_SECS: u64 = 3600;

// ── Defauts moderation ──
pub(super) const DEFAULT_BAN_CLEANUP_MINUTES: u64 = 1;
pub(super) const DEFAULT_SEND_REMINDERS_SECS: u64 = 30;

// Phase 5 — Tickets close inactifs : tick 30 min (meme cadence que
// l'ancienne boucle bot).
pub(super) const DEFAULT_TICKETS_CLOSE_INACTIVE_SECS: u64 = 1800;

// Phase 5I — Tickets SLA escalation : tick 5 min.
pub(super) const DEFAULT_TICKETS_SLA_CHECK_SECS: u64 = 300;

// Phase 5F — Quarantine kick : tick 15s (l'ancienne boucle etait a 30s
// mais avec une fenetre captcha typique de 5min, 15s = bonne reactivite).
pub(super) const DEFAULT_QUARANTINE_KICK_CHECK_SECS: u64 = 15;

// Phase 5G — Lockdown expire : tick 15s (meme cadence que la boucle
// d'origine cote bot).
pub(super) const DEFAULT_LOCKDOWN_EXPIRE_CHECK_SECS: u64 = 15;

// Phase 5H — Slowmode security expire : tick 15s.
pub(super) const DEFAULT_SLOWMODE_EXPIRE_CHECK_SECS: u64 = 15;

// ── Defauts moderation age-unban ──
/// Auto-deban verification d'age : cadence mensuelle (30 j) par defaut.
pub(super) const DEFAULT_AGE_UNBAN_INTERVAL_SECS: u64 = 30 * 24 * SECS_PER_HOUR;

// ── Defauts automod ──
/// Cloture des votes de moderation a echeance : tick 60s. CHEMIN CRITIQUE —
/// seule voie qui ferme les cartes de vote a leur deadline.
pub(super) const DEFAULT_AUTOMOD_CLOSE_VOTES_SECS: u64 = SECS_PER_MINUTE;
/// Suppression des cartes closes vieilles de > 1 mois : tick 24h.
pub(super) const DEFAULT_AUTOMOD_CLEANUP_CARDS_SECS: u64 = 24 * SECS_PER_HOUR;

// ── Defauts classement mensuel (analytics) ──
/// Check horaire ; l'API gate sur le passage de mois.
pub(super) const DEFAULT_MONTHLY_RANKING_CHECK_SECS: u64 = SECS_PER_HOUR;

// ── Defauts guild_backup (auto-backup periodique) ──
/// Cadence de VERIFICATION du worker (30 min). L'intervalle FIN est par guild
/// (`auto_backup_interval_hours`, defaut 24h) lu dans bot_guild_config.
pub(super) const DEFAULT_GUILD_BACKUP_AUTO_CHECK_SECS: u64 = 30 * SECS_PER_MINUTE;
