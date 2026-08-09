//! Scheduler central : enregistre tous les jobs periodiques avec leur
//! intervalle et delegue l'execution a `spawn_periodic` (impl commune
//! qui gere shutdown, panic catch, log lifecycle, metrics).
//!
//! Lecture de ce fichier = inventaire complet de ce que fait le worker.
//! Ajouter un job = ajouter une section ici + creer le module dans
//! `domains/{domain}/{job}.rs`.
//!
//! Note sur le `worker_name` passe a `spawn_periodic` et a
//! `is_worker_enabled` : on conserve les **noms d'origine par feature**
//! (cache-worker, audit-cache-worker, ...) plutot que de tout mettre
//! "sentinel-worker". Raison : les toggles `bot_guild_config` existants
//! sont indexes sur ces noms. Les changer obligerait a une migration DB
//! et casserait les configs guild deja en place.

use sqlx::PgPool;
use tokio::sync::watch;
use tracing::info;

use platform_common_worker::spawn_periodic;

use crate::config::{CleanupConfig, WorkerConfig};
use crate::domains;

const WORKER_NAME: &str = "sentinel-worker";

pub fn start(
    config: &WorkerConfig,
    pool: PgPool,
    redis_client: redis::Client,
    shutdown: watch::Receiver<bool>,
) {
    let api_url = config.api_url.clone();

    // ─────────────────────────────────────────────────────────────
    // Domaine : cleanup (porte de l'ancien cleanup-worker)
    // ─────────────────────────────────────────────────────────────
    {
        let cfg = CleanupConfig::from(config);
        spawn_periodic(
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
            spawn_periodic(
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

    // ─────────────────────────────────────────────────────────────
    // Domaine : automod — cloture des votes de moderation a echeance
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "automod_close_votes",
        config.automod_close_votes_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "automod-bot",
        move |pool| Box::pin(async move { domains::automod::close_votes::run(&pool).await }),
    );

    // Domaine : automod — suppression des cartes closes vieilles de > 1 mois
    // (24h). Le bot supprime le message Discord via event ; la review + le
    // transcript restent en DB (trace web conservee).
    spawn_periodic(
        "automod_cleanup_cards",
        config.automod_cleanup_cards_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "automod-bot",
        move |pool| Box::pin(async move { domains::automod::cleanup_cards::run(&pool).await }),
    );

    {
        let redis = redis_client.clone();
        spawn_periodic(
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
        spawn_periodic(
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
        spawn_periodic(
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

        spawn_periodic(
            "refresh_leaderboards",
            config.leaderboards_refresh_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            |pool| Box::pin(async move { domains::cache::refresh_leaderboards::run(&pool).await }),
        );

        spawn_periodic(
            "sync_user_cache",
            config.user_cache_sync_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "cache",
            |pool| Box::pin(async move { domains::cache::sync_user_cache::run(&pool).await }),
        );

        spawn_periodic(
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
        spawn_periodic(
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
        domains::monitoring::check_services::start(redis_client.clone(), cfg);
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : analytics (snapshots quotidien + horaire)
    // Porte de l'ancien analytics-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "daily_snapshot",
        config.daily_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::daily_snapshot::run(&pool).await }),
    );
    spawn_periodic(
        "hourly_snapshot",
        config.hourly_snapshot_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::hourly_snapshot::run(&pool).await }),
    );
    spawn_periodic(
        "analytics_retention_cleanup",
        config.analytics_retention_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "analytics",
        |pool| Box::pin(async move { domains::analytics::retention_cleanup::run(&pool).await }),
    );
    spawn_periodic(
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
    spawn_periodic(
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
        spawn_periodic(
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

    // ─────────────────────────────────────────────────────────────
    // Domaine : temp_roles (expiration des roles temporaires)
    // Porte de l'ancien temp-roles-worker.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
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
        spawn_periodic(
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
        spawn_periodic(
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
        spawn_periodic(
            "sync_discord_audit_logs",
            config.audit_sync_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "audit-bot",
            move |pool| {
                let token = token.clone();
                Box::pin(async move {
                    domains::discord_audit_sync::sync_discord_audit_logs::run(&pool, &token).await
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
        let timeout = config.ai_job_timeout_secs;
        let batch_size = config.ai_batch_size;
        spawn_periodic(
            "drain_ai_jobs",
            config.ai_poll_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ai",
            move |pool| {
                let redis = redis.clone();
                let api = api.clone();
                Box::pin(async move {
                    domains::ai::drain_ai_jobs::run(&pool, &redis, &api, timeout, batch_size).await
                })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Domaine : announcements (publication horaire alignee)
    // Porte de l'ancien announcement-worker. Structure custom (boucle
    // alignee sur HH:00:00 UTC).
    // ─────────────────────────────────────────────────────────────
    domains::announcements::publish_due::start(
        api_url.clone(),
        redis_client.clone(),
        config.announcement_publish_interval_secs,
    );
    spawn_periodic(
        "announcements_retention_cleanup",
        config.announcements_retention_check_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "announcements",
        |pool| Box::pin(async move { domains::announcements::retention_cleanup::run(&pool).await }),
    );

    // ─────────────────────────────────────────────────────────────
    // Domaine : moderation (conduit, bans, propositions, rappels)
    // Porte de l'ancien moderation-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic(
        "cleanup_bans",
        config.ban_cleanup_interval_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "moderation-bot",
        |pool| Box::pin(async move { domains::moderation::cleanup_bans::run(&pool).await }),
    );
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "send_reminders",
            config.send_reminders_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "moderation-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::moderation::send_reminders::run(&pool, &redis).await },
                )
            },
        );
    }
    // BUG #1/#2 — Auto-unban des bans temporaires a l'expiration. Chemin
    // d'enforcement reel (le DM "early" ne leve aucun ban). Independant de
    // send_reminders : couvre aussi les bans courts. Meme granularite que les
    // rappels (intervalle send_reminders).
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_temp_bans",
            config.send_reminders_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "moderation-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::moderation::expire_temp_bans::run(&pool, &redis).await },
                )
            },
        );
    }
    // Ban en sursis : bannit definitivement les sursis dont le delai d'appel
    // est ecoule -> event sursis_ban consomme par le moderation-bot.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "sursis_expire",
            config.send_reminders_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "moderation-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::moderation::sursis_expire::run(&pool, &redis).await },
                )
            },
        );
    }
    // Auto-deban verification d'age (cadence mensuelle) : leve les bans
    // age_verification_bans echus -> event age_ban_lift consomme par le bot.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "age_unban",
            config.age_unban_interval_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "welcome-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::moderation::age_unban::run(&pool, &redis).await })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 5I — Tickets SLA escalation (toutes categories sauf
    // appel_sanction qui est gere par appeal_sla::escalate_appeal_sla).
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "escalate_ticket_sla",
            config.tickets_sla_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ticket-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::tickets::escalate_sla::run(&pool, &redis).await })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 5 — Domaine tickets : fermeture auto des tickets inactifs.
    // Avant : boucle 30min dans le bot. Maintenant : worker UPDATE
    // status='closed' + XADD event 'ticket_auto_closed' que le bot
    // consume pour le menage Discord (notification + delete channel).
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "close_inactive_tickets",
            config.tickets_close_inactive_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "ticket-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move { domains::tickets::close_inactive::run(&pool, &redis).await })
            },
        );
    }

    // ─────────────────────────────────────────────────────────────
    // Phase 5F — Domaine security : kick auto des quarantaines expirees
    // (captcha non valide). Le bot publie via API a chaque mise en
    // quarantaine, ce job claim les expirees et XADD quarantine_expired.
    // ─────────────────────────────────────────────────────────────
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "kick_expired_quarantine",
            config.quarantine_kick_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "security-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(async move {
                    domains::security::kick_expired_quarantine::run(&pool, &redis).await
                })
            },
        );
    }

    // Phase 5G — Lockdown auto-revert : worker scanne les expires
    // et publie un event avec le JSON des saved_states. Le bot
    // desserialise et restaure les permissions Discord.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_lockdown",
            config.lockdown_expire_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "security-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::security::expire_lockdown::run(&pool, &redis).await },
                )
            },
        );
    }

    // Phase 5H — Slowmode security auto-revert.
    {
        let redis = redis_client.clone();
        spawn_periodic(
            "expire_slowmode",
            config.slowmode_expire_check_secs,
            pool.clone(),
            shutdown.clone(),
            api_url.clone(),
            "security-bot",
            move |pool| {
                let redis = redis.clone();
                Box::pin(
                    async move { domains::security::expire_slowmode::run(&pool, &redis).await },
                )
            },
        );
    }

    // Phases suivantes : slowmode automod (meme pattern, ~150 lignes).
    // voice-afk + progression voice tick + tickets SLA dependent
    // d'etat live populé par events Discord -> rester dans le bot.

    // Variables inutilisees a ce stade.
    let _ = (pool, shutdown, redis_client, api_url);
}

