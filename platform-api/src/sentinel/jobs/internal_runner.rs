use async_trait::async_trait;
use platform_core::sentinel::ports::inbound::system::run_internal_job::{
    InternalJobOutcome, RunInternalJobUseCase,
};

use super::{
    ai, appeal_sla, audit_cache, cache, cleanup, discord_audit_sync, export, guild_backup,
    moderation, security, temp_roles, tickets,
};

pub struct InternalJobRunner {
    pool: sqlx::PgPool,
    redis: redis::Client,
    http: reqwest::Client,
}

impl InternalJobRunner {
    pub fn new(pool: sqlx::PgPool, redis: redis::Client) -> Self {
        Self {
            pool,
            redis,
            http: reqwest::Client::new(),
        }
    }

    async fn execute(&self, job: &str) -> Result<(), String> {
        let redis = self
            .redis
            .get_connection_manager()
            .await
            .map_err(|error| error.to_string())?;
        match job {
            "cleanup-old-data" => {
                cleanup::cleanup_old_data::run(
                    &self.pool,
                    &cleanup::CleanupConfig {
                        voice_sessions_retention_days: env_i64("VOICE_SESSIONS_RETENTION_DAYS", 90),
                        logs_retention_days: env_i64("LOGS_RETENTION_DAYS", 30),
                        closed_tickets_retention_days: env_i64(
                            "CLOSED_TICKETS_RETENTION_DAYS",
                            180,
                        ),
                    },
                )
                .await
            }
            "vacuum-tables" => cleanup::vacuum_tables::run(&self.pool).await,
            "warm-analytics" => cache::warm_analytics::run(&self.pool, &redis).await,
            "warm-dashboard" => cache::warm_dashboard::run(&self.pool, &redis).await,
            "warm-voice-stats" => cache::warm_voice_stats::run(&self.pool, &redis).await,
            "refresh-leaderboards" => cache::refresh_leaderboards::run(&self.pool).await,
            "sync-user-cache" => cache::sync_user_cache::run(&self.pool).await,
            "manage-partitions" => cache::manage_partitions::run(&self.pool).await,
            "refresh-watched-users" => {
                audit_cache::refresh_watched_users::run(
                    &self.pool,
                    &redis,
                    env_i64("WATCHED_USERS_QUERY_LIMIT", 10_000).clamp(100, 100_000),
                )
                .await
            }
            "cleanup-bans" => moderation::cleanup_bans::run(&self.pool).await,
            "send-reminders" => moderation::send_reminders::run(&self.pool, &redis).await,
            "expire-temp-bans" => moderation::expire_temp_bans::run(&self.pool, &redis).await,
            "age-unban" => moderation::age_unban::run(&self.pool, &redis).await,
            "kick-expired-quarantine" => {
                security::kick_expired_quarantine::run(&self.pool, &redis).await
            }
            "remind-quarantine-rules" => {
                security::remind_quarantine_rules::run(&self.pool, &redis).await
            }
            "expire-lockdown" => security::expire_lockdown::run(&self.pool, &redis).await,
            "expire-slowmode" => security::expire_slowmode::run(&self.pool, &redis).await,
            "expire-temp-roles" => temp_roles::expire_temp_roles::run(&self.pool, &redis).await,
            "guild-backup-auto" => guild_backup::auto_backup::run(&self.pool, &redis).await,
            "escalate-appeal-sla" => appeal_sla::escalate_appeal_sla::run(&self.pool, &redis).await,
            "drain-export-jobs" => {
                export::drain_export_jobs::run(
                    &self.pool,
                    env_i64("MAX_ROWS_PER_EXPORT", 50_000).clamp(1, 1_000_000),
                    env_i64("EXPORT_PROCESSING_TIMEOUT_SECS", 300).max(1),
                )
                .await
            }
            "sync-discord-audit-logs" => {
                discord_audit_sync::sync_discord_audit_logs::run(
                    &self.pool,
                    &self.http,
                    &std::env::var("SENTINEL_DISCORD_TOKEN").unwrap_or_default(),
                )
                .await
            }
            "drain-ai-jobs" => {
                let mut redis = redis.clone();
                ai::drain_ai_jobs::run(
                    &self.pool,
                    &mut redis,
                    &self.http,
                    &std::env::var("SENTINEL_API_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:3000".into()),
                    env_u64("AI_JOB_TIMEOUT", 120).max(1),
                    env_i64("AI_BATCH_SIZE", 5).clamp(1, 100) as i32,
                )
                .await
            }
            "escalate-ticket-sla" => tickets::escalate_sla::run(&self.pool, &redis).await,
            "close-inactive-tickets" => tickets::close_inactive::run(&self.pool, &redis).await,
            _ => Err("unknown internal job".into()),
        }
    }
}

#[async_trait]
impl RunInternalJobUseCase for InternalJobRunner {
    async fn run(&self, job: &str) -> Result<InternalJobOutcome, String> {
        let lock = format!("sentinel:{job}");
        crate::shared::job_lock::run(&self.pool, &lock, || self.execute(job))
            .await
            .map(|outcome| match outcome {
                Some(()) => InternalJobOutcome::Executed,
                None => InternalJobOutcome::Locked,
            })
    }
}

fn env_i64(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
