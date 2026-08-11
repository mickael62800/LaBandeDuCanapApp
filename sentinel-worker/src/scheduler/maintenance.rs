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
    // Domaine : cleanup (porte de l'ancien cleanup-worker)
    // ─────────────────────────────────────────────────────────────
    {
        let cfg = CleanupConfig::from(config.as_ref());
        spawn_periodic!(
            "cleanup_old_data",
            config.cleanup_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            WORKER_NAME,
            move |pool| {
                let cfg = cfg.clone();
                Box::pin(async move { domains::cleanup::cleanup_old_data::run(&pool, &cfg).await })
            },
        );

        if config.vacuum_enabled {
            spawn_periodic!(
                "vacuum_tables",
                config.vacuum_interval_secs,
                pool.clone(),
                shutdown.clone(),
                api_url.clone(),
                WORKER_NAME,
                |pool| Box::pin(async move { domains::cleanup::vacuum_tables::run(&pool).await }),
            );
        } else {
            info!("VACUUM desactive par configuration");
        }
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : cache (warm Redis pour analytics, dashboard, voice)
    // Porte de l'ancien cache-worker.
    // ─────────────────────────────────────────────────────────────

    {
        let redis = redis_client.clone();
        spawn_periodic!(
            "warm_analytics",
            config.analytics_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::cache::warm_analytics::run(&pool, &redis).await })
            },
        );

        let redis = redis_client.clone();
        spawn_periodic!(
            "warm_dashboard",
            config.dashboard_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::cache::warm_dashboard::run(&pool, &redis).await })
            },
        );

        let redis = redis_client.clone();
        spawn_periodic!(
            "warm_voice_stats",
            config.voice_stats_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::cache::warm_voice_stats::run(&pool, &redis).await })
            },
        );

        spawn_periodic!(
            "refresh_leaderboards",
            config.leaderboards_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            |pool| Box::pin(async move { domains::cache::refresh_leaderboards::run(&pool).await }),
        );

        spawn_periodic!(
            "sync_user_cache",
            config.user_cache_sync_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            |pool| Box::pin(async move { domains::cache::sync_user_cache::run(&pool).await }),
        );

        spawn_periodic!(
            "manage_partitions",
            config.partition_manager_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            |pool| Box::pin(async move { domains::cache::manage_partitions::run(&pool).await }),
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : audit_cache (refresh watched_users en Redis)
    // Porte de l'ancien audit-cache-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        let watched_users_query_limit = config.watched_users_query_limit;
        spawn_periodic!(
            "refresh_watched_users",
            config.audit_cache_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "audit-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::audit_cache::refresh_watched_users::run(
                        &pool,
                        &redis,
                        watched_users_query_limit,
                    )
                    .await
                })
            },
        );
    }
}
