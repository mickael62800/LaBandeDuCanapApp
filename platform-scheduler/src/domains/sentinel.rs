use serde::Deserialize;

use crate::config::DomainConfig;

#[derive(Deserialize)]
struct JobReport {
    job: String,
    processed: usize,
    errors: usize,
}

pub fn start(config: DomainConfig) {
    let client = config.client.clone();
    crate::schedule::spawn_interval(
        "sentinel.sursis-expire",
        crate::schedule::env_u64("SENTINEL_SURSIS_EXPIRE_INTERVAL_SECS", 60),
        move || {
            let client = client.clone();
            async move {
                let report: JobReport = client
                    .post_json("/api/moderation/internal/jobs/sursis-expire")
                    .await?;
                tracing::info!(
                    job = report.job,
                    processed = report.processed,
                    errors = report.errors,
                    "job Sentinel execute"
                );
                Ok(())
            }
        },
    );

    for (name, path, interval) in [
        (
            "sentinel.analytics-daily",
            "/api/analytics/snapshot/daily",
            env("DAILY_SNAPSHOT_INTERVAL", 1) * 3_600,
        ),
        (
            "sentinel.analytics-hourly",
            "/api/analytics/snapshot/hourly",
            env("HOURLY_SNAPSHOT_INTERVAL", 60) * 60,
        ),
        (
            "sentinel.analytics-retention",
            "/api/analytics/retention-cleanup",
            env("ANALYTICS_RETENTION_CHECK", 86_400),
        ),
        (
            "sentinel.analytics-top-users",
            "/api/analytics/publish-top-users",
            env("TOP_USERS_PUBLISH_CHECK", 3_600),
        ),
        (
            "sentinel.analytics-monthly-ranking",
            "/api/analytics/publish-monthly-ranking",
            env("MONTHLY_RANKING_CHECK_SECS", 3_600),
        ),
        (
            "sentinel.automod-cleanup-cards",
            "/api/automod/cleanup-expired-cards",
            env("AUTOMOD_CLEANUP_CARDS_SECS", 86_400),
        ),
        (
            "sentinel.automod-close-votes",
            "/api/automod/internal/jobs/close-votes",
            env("AUTOMOD_CLOSE_VOTES_SECS", 60),
        ),
        (
            "sentinel.announcements-retention",
            "/api/announcements/internal/retention-cleanup",
            env("ANNOUNCEMENTS_RETENTION_CHECK", 86_400),
        ),
        (
            "sentinel.announcements-publish-due",
            "/api/announcements/internal/jobs/publish-due",
            env("ANNOUNCEMENT_PUBLISH_INTERVAL_SECS", 3_600),
        ),
        (
            "sentinel.cleanup-old-data",
            "/api/internal/jobs/cleanup-old-data",
            env("CLEANUP_INTERVAL_HOURS", 1) * 3_600,
        ),
        (
            "sentinel.warm-analytics",
            "/api/internal/jobs/warm-analytics",
            env("ANALYTICS_REFRESH_INTERVAL", 300),
        ),
        (
            "sentinel.warm-dashboard",
            "/api/internal/jobs/warm-dashboard",
            env("DASHBOARD_REFRESH_INTERVAL", 600),
        ),
        (
            "sentinel.warm-voice-stats",
            "/api/internal/jobs/warm-voice-stats",
            env("VOICE_STATS_REFRESH_INTERVAL", 3_600),
        ),
        (
            "sentinel.refresh-leaderboards",
            "/api/internal/jobs/refresh-leaderboards",
            env("LEADERBOARDS_REFRESH_INTERVAL", 300),
        ),
        (
            "sentinel.sync-user-cache",
            "/api/internal/jobs/sync-user-cache",
            env("USER_CACHE_SYNC_INTERVAL", 900),
        ),
        (
            "sentinel.manage-partitions",
            "/api/internal/jobs/manage-partitions",
            env("PARTITION_MANAGER_INTERVAL", 86_400),
        ),
        (
            "sentinel.refresh-watched-users",
            "/api/internal/jobs/refresh-watched-users",
            env("AUDIT_CACHE_REFRESH_INTERVAL", 60),
        ),
        (
            "sentinel.cleanup-bans",
            "/api/internal/jobs/cleanup-bans",
            env("BAN_CLEANUP_INTERVAL", 1) * 60,
        ),
        (
            "sentinel.send-reminders",
            "/api/internal/jobs/send-reminders",
            env("SEND_REMINDERS_INTERVAL", 30),
        ),
        (
            "sentinel.expire-temp-bans",
            "/api/internal/jobs/expire-temp-bans",
            env("SEND_REMINDERS_INTERVAL", 30),
        ),
        (
            "sentinel.age-unban",
            "/api/internal/jobs/age-unban",
            env("AGE_UNBAN_INTERVAL", 2_592_000),
        ),
        (
            "sentinel.kick-expired-quarantine",
            "/api/internal/jobs/kick-expired-quarantine",
            env("QUARANTINE_KICK_CHECK_SECS", 15),
        ),
        // Rappel d'accepter le reglement avant expulsion. Meme cadence que
        // l'expulsion : le rappel doit partir AVANT elle, un balayage plus lent
        // laisserait passer des echeances courtes.
        (
            "sentinel.remind-quarantine-rules",
            "/api/internal/jobs/remind-quarantine-rules",
            env("QUARANTINE_REMINDER_CHECK_SECS", 15),
        ),
        (
            "sentinel.expire-lockdown",
            "/api/internal/jobs/expire-lockdown",
            env("LOCKDOWN_EXPIRE_CHECK_SECS", 15),
        ),
        (
            "sentinel.expire-slowmode",
            "/api/internal/jobs/expire-slowmode",
            env("SLOWMODE_EXPIRE_CHECK_SECS", 15),
        ),
        (
            "sentinel.expire-temp-roles",
            "/api/internal/jobs/expire-temp-roles",
            env("TEMP_ROLES_SCAN_INTERVAL", 60),
        ),
        (
            "sentinel.guild-backup-auto",
            "/api/internal/jobs/guild-backup-auto",
            env("GUILD_BACKUP_AUTO_CHECK_INTERVAL", 1_800),
        ),
        (
            "sentinel.escalate-appeal-sla",
            "/api/internal/jobs/escalate-appeal-sla",
            env("APPEAL_SLA_SCAN_INTERVAL", 120),
        ),
        (
            "sentinel.drain-export-jobs",
            "/api/internal/jobs/drain-export-jobs",
            env("EXPORT_SCAN_INTERVAL", 5),
        ),
        (
            "sentinel.sync-discord-audit-logs",
            "/api/internal/jobs/sync-discord-audit-logs",
            env("AUDIT_SYNC_INTERVAL", 300),
        ),
        (
            "sentinel.drain-ai-jobs",
            "/api/internal/jobs/drain-ai-jobs",
            env("AI_POLL_INTERVAL", 2),
        ),
        (
            "sentinel.escalate-ticket-sla",
            "/api/internal/jobs/escalate-ticket-sla",
            env("TICKETS_SLA_CHECK_INTERVAL", 300),
        ),
        (
            "sentinel.close-inactive-tickets",
            "/api/internal/jobs/close-inactive-tickets",
            env("TICKETS_CLOSE_INACTIVE_INTERVAL", 1_800),
        ),
    ] {
        let client = config.client.clone();
        crate::schedule::spawn_interval(name, interval, move || {
            let client = client.clone();
            async move {
                let report: serde_json::Value = client.post_json(path).await?;
                tracing::info!(job = name, %report, "job Sentinel execute");
                Ok(())
            }
        });
    }

    if env_bool("VACUUM_ENABLED", true) {
        let client = config.client.clone();
        crate::schedule::spawn_interval(
            "sentinel.vacuum-tables",
            env("VACUUM_INTERVAL_HOURS", 24) * 3_600,
            move || {
                let client = client.clone();
                async move {
                    let _: serde_json::Value =
                        client.post_json("/api/internal/jobs/vacuum-tables").await?;
                    Ok(())
                }
            },
        );
    }
}

fn env(name: &str, default: u64) -> u64 {
    crate::schedule::env_u64(name, default)
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
