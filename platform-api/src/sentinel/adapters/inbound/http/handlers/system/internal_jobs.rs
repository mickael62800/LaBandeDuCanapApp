use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::sentinel::bootstrap::state::InternalJobsState;
use crate::sentinel::jobs::{
    ai, appeal_sla, audit_cache, cache, cleanup, discord_audit_sync, export, guild_backup,
    moderation, security, temp_roles, tickets,
};

pub async fn run(
    State(state): State<InternalJobsState>,
    Path(job): Path<String>,
) -> impl IntoResponse {
    let redis = match state.redis_client.get_connection_manager().await {
        Ok(redis) => redis,
        Err(error) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({"job": job, "error": error.to_string()})),
            )
        }
    };

    let lock_name = format!("sentinel:{job}");
    let result = platform_common_api::job_lock::run(&state.pg_pool, &lock_name, || async {
        match job.as_str() {
            "cleanup-old-data" => {
                cleanup::cleanup_old_data::run(
                    &state.pg_pool,
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
            "vacuum-tables" => cleanup::vacuum_tables::run(&state.pg_pool).await,
            "warm-analytics" => cache::warm_analytics::run(&state.pg_pool, &redis).await,
            "warm-dashboard" => cache::warm_dashboard::run(&state.pg_pool, &redis).await,
            "warm-voice-stats" => cache::warm_voice_stats::run(&state.pg_pool, &redis).await,
            "refresh-leaderboards" => cache::refresh_leaderboards::run(&state.pg_pool).await,
            "sync-user-cache" => cache::sync_user_cache::run(&state.pg_pool).await,
            "manage-partitions" => cache::manage_partitions::run(&state.pg_pool).await,
            "refresh-watched-users" => {
                audit_cache::refresh_watched_users::run(
                    &state.pg_pool,
                    &redis,
                    env_i64("WATCHED_USERS_QUERY_LIMIT", 10_000).clamp(100, 100_000),
                )
                .await
            }
            "cleanup-bans" => moderation::cleanup_bans::run(&state.pg_pool).await,
            "send-reminders" => moderation::send_reminders::run(&state.pg_pool, &redis).await,
            "expire-temp-bans" => moderation::expire_temp_bans::run(&state.pg_pool, &redis).await,
            "age-unban" => moderation::age_unban::run(&state.pg_pool, &redis).await,
            "kick-expired-quarantine" => {
                security::kick_expired_quarantine::run(&state.pg_pool, &redis).await
            }
            "expire-lockdown" => security::expire_lockdown::run(&state.pg_pool, &redis).await,
            "expire-slowmode" => security::expire_slowmode::run(&state.pg_pool, &redis).await,
            "expire-temp-roles" => temp_roles::expire_temp_roles::run(&state.pg_pool, &redis).await,
            "guild-backup-auto" => guild_backup::auto_backup::run(&state.pg_pool, &redis).await,
            "escalate-appeal-sla" => {
                appeal_sla::escalate_appeal_sla::run(&state.pg_pool, &redis).await
            }
            "drain-export-jobs" => {
                export::drain_export_jobs::run(
                    &state.pg_pool,
                    env_i64("MAX_ROWS_PER_EXPORT", 50_000).clamp(1, 1_000_000),
                    env_i64("EXPORT_PROCESSING_TIMEOUT_SECS", 300).max(1),
                )
                .await
            }
            "sync-discord-audit-logs" => {
                discord_audit_sync::sync_discord_audit_logs::run(
                    &state.pg_pool,
                    &reqwest::Client::new(),
                    &std::env::var("SENTINEL_DISCORD_TOKEN").unwrap_or_default(),
                )
                .await
            }
            "drain-ai-jobs" => {
                let mut ai_redis = redis.clone();
                ai::drain_ai_jobs::run(
                    &state.pg_pool,
                    &mut ai_redis,
                    &reqwest::Client::new(),
                    &std::env::var("SENTINEL_API_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:3000".into()),
                    env_u64("AI_JOB_TIMEOUT", 120).max(1),
                    env_i64("AI_BATCH_SIZE", 5).clamp(1, 100) as i32,
                )
                .await
            }
            "escalate-ticket-sla" => tickets::escalate_sla::run(&state.pg_pool, &redis).await,
            "close-inactive-tickets" => tickets::close_inactive::run(&state.pg_pool, &redis).await,
            _ => Err("unknown internal job".into()),
        }
    })
    .await;

    match result {
        Ok(Some(())) => (
            StatusCode::OK,
            Json(json!({"job": job, "processed": 1, "errors": 0})),
        ),
        Ok(None) => (
            StatusCode::ACCEPTED,
            Json(json!({"job": job, "processed": 0, "errors": 0, "locked": true})),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"job": job, "processed": 0, "errors": 1, "error": error})),
        ),
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
