//! Config globale du worker unifie.
//!
//! Chaque domaine ajoute ses propres champs ici (intervalles, retentions,
//! flags). Les valeurs viennent par defaut du code, peuvent etre override
//! par variables d'env, et finalement par la table `bot_guild_config`
//! (cle `sentinel-worker`). Pattern aligne sur les anciens workers.

mod defaults;
mod loader;

#[derive(Clone)]
pub struct WorkerConfig {
    pub database_url: String,
    pub redis_url: String,
    pub api_url: String,

    // ── Cleanup ──
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
    pub cleanup_interval_secs: u64,
    pub vacuum_enabled: bool,
    pub vacuum_interval_secs: u64,

    // ── Cache (warm Redis) ──
    pub analytics_refresh_secs: u64,
    pub dashboard_refresh_secs: u64,
    pub voice_stats_refresh_secs: u64,
    pub leaderboards_refresh_secs: u64,
    pub user_cache_sync_secs: u64,
    pub partition_manager_secs: u64,

    // ── Audit cache ──
    pub audit_cache_refresh_secs: u64,
    pub watched_users_query_limit: i64,

    // ── Monitoring ──
    pub api_key: String,
    pub monitor_check_interval_secs: u64,

    // ── Analytics ──
    pub daily_snapshot_interval_secs: u64,
    pub hourly_snapshot_interval_secs: u64,
    pub analytics_retention_check_secs: u64,
    pub top_users_publish_check_secs: u64,
    pub announcements_retention_check_secs: u64,

    // ── Temp roles ──
    pub temp_roles_scan_interval_secs: u64,

    // ── Appeal SLA ──
    pub appeal_sla_scan_interval_secs: u64,

    // ── Export ──
    pub export_scan_interval_secs: u64,
    pub max_rows_per_export: i64,
    pub export_processing_timeout_secs: i64,

    // ── Discord audit sync ──
    pub audit_sync_interval_secs: u64,
    pub discord_bot_token: String,

    // ── AI ──
    pub ai_poll_interval_secs: u64,
    pub ai_job_timeout_secs: u64,
    pub ai_batch_size: i32,

    // ── Announcements ──
    pub announcement_publish_interval_secs: u64,

    // ── Moderation ──
    pub ban_cleanup_interval_secs: u64,
    pub send_reminders_interval_secs: u64,
    pub age_unban_interval_secs: u64,

    // ── Tickets ──
    pub tickets_close_inactive_secs: u64,
    pub tickets_sla_check_secs: u64,

    // ── Security ──
    pub quarantine_kick_check_secs: u64,
    pub lockdown_expire_check_secs: u64,
    pub slowmode_expire_check_secs: u64,

    // ── Automod ──
    pub automod_close_votes_secs: u64,
    pub automod_cleanup_cards_secs: u64,

    // ── Classement mensuel (analytics) ──
    pub monthly_ranking_check_secs: u64,

    // ── Guild backup (auto-backup periodique) ──
    pub guild_backup_auto_check_secs: u64,
}

/// Sous-config passee aux jobs cleanup (pour ne pas leur donner toute la
/// WorkerConfig).
#[derive(Clone)]
pub struct CleanupConfig {
    pub voice_sessions_retention_days: i64,
    pub logs_retention_days: i64,
    pub closed_tickets_retention_days: i64,
}

impl From<&WorkerConfig> for CleanupConfig {
    fn from(c: &WorkerConfig) -> Self {
        Self {
            voice_sessions_retention_days: c.voice_sessions_retention_days,
            logs_retention_days: c.logs_retention_days,
            closed_tickets_retention_days: c.closed_tickets_retention_days,
        }
    }
}
