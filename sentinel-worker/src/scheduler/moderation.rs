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
    // Domaine : automod — cloture des votes de moderation a echeance
    // ─────────────────────────────────────────────────────────────
    spawn_periodic!(
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
    spawn_periodic!(
        "automod_cleanup_cards",
        config.automod_cleanup_cards_secs,
        pool.clone(),
        shutdown.clone(),
        api_url.clone(),
        "automod-bot",
        move |pool| Box::pin(async move { domains::automod::cleanup_cards::run(&pool).await }),
    );

    // ─────────────────────────────────────────────────────────────
    // Domaine : moderation (conduit, bans, propositions, rappels)
    // Porte de l'ancien moderation-worker.
    // ─────────────────────────────────────────────────────────────
    spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
        spawn_periodic!(
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
}
