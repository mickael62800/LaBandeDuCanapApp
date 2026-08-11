use super::*;

pub(super) fn register(context: &WorkerContext, tasks: &mut Vec<SupervisedTask>) {
    let config = context.config.clone();
    let pool = context.pool.clone();
    let redis_client = context.redis.clone();
    let shutdown = context.shutdown.clone();
    let api_url = config.api_url.clone();

    macro_rules! spawn_periodic {
        ($($args:tt)*) => {
            tasks.push(platform_common_worker::spawn_periodic($($args)*))
        };
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : temp_roles (expiration des roles temporaires)
    // Porte de l'ancien temp-roles-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic!(
            "expire_temp_roles",
            config.temp_roles_scan_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "temp_roles",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::temp_roles::expire_temp_roles::run(&pool, &redis).await },
                )
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : appeal_sla (escalade des appels de sanction)
    // Porte de l'ancien appeal-sla-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic!(
            "escalate_appeal_sla",
            config.appeal_sla_scan_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ticket-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::appeal_sla::escalate_appeal_sla::run(&pool, &redis).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : export (drain export_jobs)
    // Porte de l'ancien export-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let max_rows = config.max_rows_per_export;
        let export_timeout = config.export_processing_timeout_secs;
        spawn_periodic!(
            "drain_export_jobs",
            config.export_scan_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "export",
            move |pool| {
                Box::pin(async move {
                    domains::export::drain_export_jobs::run(&pool, max_rows, export_timeout).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : discord_audit_sync (poll Discord audit-logs API)
    // Porte de l'ancien discord-audit-sync-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let token = config.discord_bot_token.clone();
        let http = context.http.standard.clone();
        spawn_periodic!(
            "sync_discord_audit_logs",
            config.audit_sync_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "audit-bot",
            move |pool| {
                let token = token.clone();
                let http = http.clone();
                Box::pin(async move {
                    domains::discord_audit_sync::sync_discord_audit_logs::run(&pool, &http, &token)
                        .await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : ai (drain ai_jobs)
    // Porte de l'ancien ai-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        let api = api_url.clone();
        let http = context.http.long_running.clone();
        let timeout = config.ai_job_timeout_secs;
        let batch_size = config.ai_batch_size;
        spawn_periodic!(
            "drain_ai_jobs",
            config.ai_poll_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ai",
            move |pool| {
                let redis = redis.clone();
                let api = api.clone();
                let http = http.clone();
                Box::pin(async move {
                    let mut redis = redis;
                    domains::ai::drain_ai_jobs::run(
                        &pool, &mut redis, &http, &api, timeout, batch_size,
                    )
                    .await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : announcements (publication horaire alignee)
    // Porte de l'ancien announcement-worker. Structure custom (boucle
    // alignee sur HH:00:00 UTC).
    // ─────────────────────────────────────────────────────────────
    tasks.push(domains::announcements::publish_due::start(
        context.http.standard.clone(),
        api_url.clone(),
        redis_client.clone(),
        config.announcement_publish_interval_secs,
        shutdown.clone(),
    ));
    spawn_periodic!(
        "announcements_retention_cleanup",
        config.announcements_retention_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "announcements",
        |pool| Box::pin(async move { domains::announcements::retention_cleanup::run(&pool).await }),
    );
}
