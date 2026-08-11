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
    // Domaine : monitoring (surveillance bots/workers online)
    // Porte de l'ancien monitoring-worker. Structure differente :
    // boucle stateful (track previous_online), pas un simple
    // spawn_periodic. On delegue a son propre `start()`.
    // ─────────────────────────────────────────────────────────────
    {
        let cfg = domains::monitoring::MonitorConfig {
            api_url: api_url.clone(),
            api_key: config.api_key.clone(),
            check_interval_secs: config.monitor_check_interval_secs,
        };
        tasks.push(domains::monitoring::check_services::start(
            context.http.standard.clone(),
            redis_client.clone(),
            cfg,
            shutdown.clone(),
        ));
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : analytics (snapshots quotidien + horaire)
    // Porte de l'ancien analytics-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic!(
        "daily_snapshot",
        config.daily_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::daily_snapshot::run(&pool).await }),
    );
    spawn_periodic!(
        "hourly_snapshot",
        config.hourly_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::hourly_snapshot::run(&pool).await }),
    );
    spawn_periodic!(
        "analytics_retention_cleanup",
        config.analytics_retention_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::retention_cleanup::run(&pool).await }),
    );
    spawn_periodic!(
        "publish_top_users",
        config.top_users_publish_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::publish_top_users::run(&pool).await }),
    );
    // Classement mensuel (texte/vocal/global) : check horaire. L'API gate sur
    // le passage de mois (baseline du mois courant deja posee -> no-op), donc
    // un tick horaire publie au plus tot apres le 1er du mois sans spammer.
    spawn_periodic!(
        "publish_monthly_ranking",
        config.monthly_ranking_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| {
            Box::pin(async move { domains::analytics::publish_monthly_ranking::run(&pool).await })
        },
    );

    // ─────────────────────────────────────────────────────────────
    // Domaine : guild_backup (auto-backup periodique)
    // Le worker publie `guild_backup:capture_requested` pour les guilds dont
    // l'intervalle configure est ecoule ; le bot fait la capture reelle. La
    // cadence ici n'est qu'une frequence de VERIFICATION (30 min par defaut) —
    // l'intervalle FIN est par guild (`auto_backup_interval_hours`).
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic!(
            "guild_backup_auto",
            config.guild_backup_auto_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "guild-backup-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::guild_backup::auto_backup::run(&pool, &redis).await },
                )
            },
        );
    }
}
