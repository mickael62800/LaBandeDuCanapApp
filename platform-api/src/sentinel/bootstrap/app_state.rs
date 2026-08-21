//! Assemblage complet de l'AppState : tous les repos + services (DI).

mod imports;
use imports::*;

/// Construit l'etat complet de l'application (repositories et services).
///
/// Cette fonction est la composition root : elle choisit les implementations
/// concretes des ports du core. Les handlers ne construisent jamais de
/// repository eux-memes. PostgreSQL et Redis sont partages via des handles
/// clones ; les caches accelerent les lectures mais ne remplacent pas la base.
pub async fn build_app_state(
    config: &AppConfig,
    pg_pool: sqlx::PgPool,
    redis_client: redis::Client,
) -> AppState {
    // ── Adapters sortants ──
    let rule_repo = Arc::new(PgRuleRepository::new(pg_pool.clone()));
    let infraction_repo = Arc::new(PgInfractionRepository::new(pg_pool.clone()));
    let ticket_repo = Arc::new(PgTicketRepository::new(pg_pool.clone()));
    let security_repo = Arc::new(PgSecurityEventRepository::new(pg_pool.clone()));
    let moderation_repo = Arc::new(PgModerationRepository::new(pg_pool.clone()));
    let stats_repo = Arc::new(PgStatsRepository::new(pg_pool.clone()));
    let age_ban_repo = Arc::new(
        crate::sentinel::adapters::outbound::postgres::community::age_ban_repository::PgAgeBanRepository::new(
            pg_pool.clone(),
        ),
    );
    let bot_config_repo = Arc::new(PgBotConfigRepository::new(pg_pool.clone()));
    let guild_repo = Arc::new(PgGuildRepository::new(pg_pool.clone()));
    // Phase 5C — Batch writes : BatchedPgLogRepository bufferise les inserts et
    // flush via multi-row INSERT toutes les 500ms ou 100 entries.
    let log_repo = Arc::new(BatchedPgLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));

    let strike_repo = Arc::new(PgStrikeRepository::new(pg_pool.clone()));
    let cache = Arc::new(
        RedisCache::new(redis_client.clone())
            .await
            .expect("Impossible d'etablir la connexion Redis pour le cache"),
    );

    // ── Event broadcaster (Redis pub/sub → gateway WebSocket) ──
    let broadcaster = crate::sentinel::bootstrap::build_broadcaster(redis_client.clone());

    let (discord_api, inference, analyze_uc, analyze_image_uc, dataset_uc, ai_jobs_uc) =
        include!("app_state/ai.rs");

    let rules_uc = Arc::new(ManageRulesService::new(rule_repo.clone(), cache.clone()));
    let infractions_uc = Arc::new(ManageInfractionsService::new(infraction_repo.clone()));
    let tickets_uc = Arc::new(ManageTicketsService::new(
        ticket_repo.clone(),
        cache.clone(),
    ));
    // Phase 5C — Batch writes : idem que log_repo, pour les audit events.
    // Creation deplacee plus tot pour pouvoir injecter audit_logs_uc dans
    // security_uc. Les actions de moderation passent directement par leur
    // repository, dont la source de verite est aussi audit_logs.
    let audit_log_repo = Arc::new(BatchedPgAuditLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let audit_logs_uc = Arc::new(ManageAuditLogsService::new(audit_log_repo));

    // Planning communautaire : evenements et campagnes de jeu.
    let events_uc = Arc::new(
        platform_core::sentinel::application::community::manage_events_service::ManageEventsService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::postgres::community::event_repository::PgEventRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    // Vie de la communaute : annonces de recherche de joueurs, sondages,
    // membre du mois, nouvelles du site. Quatre concepts distincts mais un
    // seul public : la page de l'espace membre.
    let lfg_uc = Arc::new(
        platform_core::sentinel::application::community::manage_lfg_service::ManageLfgService::new(Arc::new(
            crate::sentinel::adapters::outbound::postgres::community::lfg_repository::PgLfgRepository::new(
                pg_pool.clone(),
            ),
        )),
    );
    let polls_uc = Arc::new(
        platform_core::sentinel::application::community::manage_polls_service::ManagePollsService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::postgres::community::poll_repository::PgPollRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );
    let spotlight_uc = Arc::new(
        platform_core::sentinel::application::community::manage_spotlight_service::ManageSpotlightService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::postgres::community::spotlight_repository::PgSpotlightRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );
    let news_uc = Arc::new(
        platform_core::sentinel::application::community::manage_news_service::ManageNewsService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::postgres::community::news_repository::PgNewsRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    // Presence en direct. Le bot publie, l'API lit : elle n'a aucun moyen de
    // savoir qui est en vocal, et fabriquer cette donnee ici serait mentir.
    let presence_uc = Arc::new(
        platform_core::sentinel::application::community::read_presence_service::ReadPresenceService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::redis_presence::RedisPresenceRepository::new(
                    redis_client.clone(),
                ),
            ),
        ),
    );

    // Detection d'anomalie de moderation (mass ban/delete/role). Le CALCUL
    // (fenetre glissante) vit dans un adapter memoire serveur ; la DECISION
    // (seuil + reset) dans le service coeur. Le bot n'agrege/ne decide plus.
    let anomaly_max_buffer = std::env::var("ANOMALY_DETECTOR_MAX_BUFFER_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500usize);
    let anomaly_eviction_target = std::env::var("ANOMALY_DETECTOR_EVICTION_TARGET")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100usize);
    let anomaly_counter = Arc::new(
        crate::sentinel::adapters::outbound::audit::in_memory_anomaly_counter::InMemoryAnomalyCounter::new(
            anomaly_max_buffer,
            anomaly_eviction_target,
        ),
    );
    let detect_anomaly_uc = Arc::new(
        platform_core::sentinel::application::audit::detect_moderation_anomaly_service::DetectModerationAnomalyService::new(
            anomaly_counter,
        ),
    );

    // Rapport hebdomadaire agrege server-side : comptage postgres par event_type
    // sur 7 jours (remonte de l'ancien WeeklyTracker RAM du bot).
    let weekly_report_uc = Arc::new(
        platform_core::sentinel::application::audit::get_weekly_report_service::GetWeeklyReportService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::postgres::audit::audit_event_counter::PgAuditEventCounter::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    let user_activity_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::audit::user_activity_repository::UserActivityRepository,
    > = Arc::new(PgUserActivityRepository::new(pg_pool.clone()));
    let welcome_config_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository,
    > = Arc::new(PgWelcomeConfigRepository::new(pg_pool.clone()));
    // Use case Welcome (Phase 3) — handlers HTTP/gRPC passent par ce port
    // inbound, jamais par le repo direct.
    let welcome_config_uc: Arc<dyn platform_core::sentinel::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase> =
        Arc::new(platform_core::sentinel::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
            welcome_config_repo.clone(),
        ));
    // Verification d'age : DECISION server-side (le bot n'execute que l'action
    // Discord). Lit la config welcome du serveur via le meme repo.
    let age_check_uc: Arc<dyn platform_core::sentinel::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase> =
        Arc::new(platform_core::sentinel::application::community::evaluate_age_declaration_service::EvaluateAgeDeclarationService::new(
            welcome_config_repo.clone(),
        ));
    // Automod reviews (sync Discord <-> web).
    let automod_review_repo: Arc<dyn platform_core::sentinel::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository> = Arc::new(
        crate::sentinel::adapters::outbound::postgres::moderation::automod_review_repository::PgAutomodReviewRepository::new(pg_pool.clone()),
    );
    let automod_adaptive_slowmode_repo: Arc<dyn platform_core::sentinel::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository> = Arc::new(
        crate::sentinel::adapters::outbound::postgres::moderation::adaptive_slowmode_repository::PgAdaptiveSlowmodeRepository::new(pg_pool.clone()),
    );
    let automod_reviews_uc: Arc<dyn platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase> =
        Arc::new(platform_core::sentinel::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
            automod_review_repo.clone(),
        ));

    // Reset complet d'un serveur (factory reset, owner-only).
    let reset_guild_uc: Arc<dyn platform_core::sentinel::ports::inbound::system::reset_guild::ResetGuildUseCase> =
        Arc::new(platform_core::sentinel::application::system::reset_guild_service::ResetGuildService::new(Arc::new(
            crate::sentinel::adapters::outbound::postgres::system::guild_reset_repository::PgGuildResetRepository::new(pg_pool.clone()),
        )));

    let watched_user_repo = Arc::new(PgWatchedUserRepository::new(pg_pool.clone()));
    let security_uc = Arc::new(
        ManageSecurityService::new(
            security_repo.clone(),
            cache.clone(),
            watched_user_repo.clone(),
            bot_config_repo.clone(),
            moderation_repo.clone(),
        )
        .with_audit_logs_uc(audit_logs_uc.clone()
            as Arc<
                dyn platform_core::sentinel::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase,
            >),
    );
    // Note : la creation de moderation_uc est differee plus bas pour pouvoir
    // injecter strikes_uc via with_strikes_uc (log_action_with_strike).
    let service_registry: Arc<
        dyn platform_core::ops::ports::outbound::service_registry::ServiceRegistry,
    > = Arc::new(
        crate::sentinel::adapters::outbound::redis_service_registry::RedisServiceRegistry::new(
            redis_client.clone(),
        ),
    );
    let stats_uc = Arc::new(ManageStatsService::new(
        stats_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
    ));
    let analytics_repo = Arc::new(PgAnalyticsRepository::new(pg_pool.clone()));
    let level_repo = Arc::new(PgLevelRepository::new(pg_pool.clone()));
    let manage_levels_usecase = Arc::new(ManageLevelsService::new(
        level_repo.clone(),
        bot_config_repo.clone(),
    ));
    let daily_activity_repo = Arc::new(PgDailyActivityRepository::new(pg_pool.clone()));
    let announcement_repo = Arc::new(crate::sentinel::adapters::outbound::postgres::community::announcement_repository::PgAnnouncementRepository::new(pg_pool.clone()));
    let announcements_uc: Arc<dyn platform_core::sentinel::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase> = Arc::new(platform_core::sentinel::application::community::manage_announcements_service::ManageAnnouncementsService::new(announcement_repo, bot_config_repo.clone()));
    let embed_repo = Arc::new(
        crate::sentinel::adapters::outbound::postgres::community::embed_repository::PgEmbedRepository::new(
            pg_pool.clone(),
        ),
    );
    let embeds_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::community::manage_embeds::ManageEmbedsUseCase,
    > = Arc::new(
        platform_core::sentinel::application::community::manage_embeds_service::ManageEmbedsService::new(
            embed_repo,
        ),
    );
    let idea_repo = Arc::new(
        crate::sentinel::adapters::outbound::postgres::community::idea_repository::PgIdeaRepository::new(
            pg_pool.clone(),
        ),
    );
    let ideas_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::community::manage_ideas::ManageIdeasUseCase,
    > = Arc::new(
        platform_core::sentinel::application::community::manage_ideas_service::ManageIdeasService::new(
            idea_repo,
        ),
    );
    let confession_repo = Arc::new(crate::sentinel::adapters::outbound::postgres::community::confession_repository::PgConfessionRepository::new(pg_pool.clone()));
    let confessions_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::community::manage_confessions::ManageConfessionsUseCase,
    > = Arc::new(
        platform_core::sentinel::application::community::manage_confessions_service::ManageConfessionsService::new(
            confession_repo,
        ),
    );
    // Evaluation server-side du risque de cible (garde-fou UX confirmation) :
    // lit le seuil `risk_recent_account_days` (moderation-bot, defaut 7j) et
    // applique la politique. Le bot ne fournit que les faits Discord.
    let assess_target_risk_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::moderation::assess_target_risk::AssessTargetRiskUseCase,
    > = Arc::new(
        platform_core::sentinel::application::moderation::assess_target_risk_service::AssessTargetRiskService::new(
            bot_config_repo.clone(),
        ),
    );
    let moderation_uc = Arc::new(ManageModerationService::new(
        moderation_repo.clone(),
        strike_repo.clone(),
        cache.clone(),
    ));
    let discord_role_repo = Arc::new(PgDiscordRoleRepository::new(pg_pool.clone()));

    // Eligibilite Community : decisions server-side (prerequis de role +
    // validation de parrainage). Lit la config via bot_config_repo ; regles
    // pures dans le domaine. Le bot ne fournit que les donnees Discord.
    let eligibility_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::community::check_eligibility::CheckEligibilityUseCase,
    > = Arc::new(
        platform_core::sentinel::application::community::check_eligibility_service::CheckEligibilityService::new(
            bot_config_repo.clone(),
        ),
    );

    // Classement mensuel : repo Postgres (deltas d'XP + baselines) + use case
    // (gates de publication, assemblage des tops, pose des baselines). Le
    // handler HTTP ne fait que RBAC + envoi Discord.
    let monthly_ranking_repo = Arc::new(
        crate::sentinel::adapters::outbound::postgres::community::monthly_ranking_repository::PgMonthlyRankingRepository::new(
            pg_pool.clone(),
        ),
    );
    let monthly_ranking_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase,
    > = Arc::new(
        platform_core::sentinel::application::community::manage_monthly_ranking_service::ManageMonthlyRankingService::new(
            bot_config_repo.clone(),
            monthly_ranking_repo,
        ),
    );

    // Snapshots analytics : repo Postgres (SQL des jobs) + use case (config par
    // guild, deltas de baseline, filtres de publication). Les handlers HTTP ne
    // font que declencher/RBAC/poster.
    let snapshot_repo = Arc::new(
        crate::sentinel::adapters::outbound::postgres::audit::snapshot_repository::PgSnapshotRepository::new(
            pg_pool.clone(),
        ),
    );
    let snapshots_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase,
    > = Arc::new(
        platform_core::sentinel::application::audit::manage_snapshots_service::ManageSnapshotsService::new(
            bot_config_repo.clone(),
            snapshot_repo,
            analytics_repo.clone(),
        ),
    );

    // Sauvegarde / restauration de serveur (guild_backup) : repo + use case.
    let snapshot_repo: Arc<dyn platform_core::sentinel::ports::outbound::guild_backup::snapshot_repository::SnapshotRepository> =
        Arc::new(
            crate::sentinel::adapters::outbound::postgres::guild_backup::snapshot_repository::PgSnapshotRepository::new(
                pg_pool.clone(),
            ),
        );
    let guild_snapshots_uc: Arc<dyn platform_core::sentinel::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase> =
        Arc::new(
            platform_core::sentinel::application::guild_backup::manage_snapshots_service::ManageGuildSnapshotsService::new(
                snapshot_repo.clone(),
            ),
        );
    // Re-attribution des roles aux membres de retour (pending_role_grants).
    let pending_role_grant_repo: Arc<dyn platform_core::sentinel::ports::outbound::guild_backup::pending_role_grant_repository::PendingRoleGrantRepository> =
        Arc::new(
            crate::sentinel::adapters::outbound::postgres::guild_backup::pending_role_grant_repository::PgPendingRoleGrantRepository::new(
                pg_pool.clone(),
            ),
        );
    let pending_role_grants_uc: Arc<dyn platform_core::sentinel::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase> =
        Arc::new(
            platform_core::sentinel::application::guild_backup::manage_pending_role_grants_service::ManagePendingRoleGrantsService::new(
                pending_role_grant_repo.clone(),
            ),
        );

    // L'OAuth Discord web (sessions, journal de login, echange de jetons) a ete
    // extrait dans la plateforme `auth-*`. Ce processus n'en garde qu'un
    // consommateur : `AppState.auth`, cf. `middleware/superadmin.rs`.

    // Quarantaine de securite : repo Postgres (SQL security_quarantine_pending) +
    // use case (reglage de la guilde et calcul de l'echeance). Le handler ne
    // fait que parse/RBAC/map.
    let quarantine_repo: Arc<dyn platform_core::sentinel::ports::outbound::system::quarantine_repository::QuarantineRepository> =
        Arc::new(crate::sentinel::adapters::outbound::postgres::system::quarantine_repository::PgQuarantineRepository::new(pg_pool.clone()));
    let quarantine_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase,
    > = Arc::new(
        platform_core::sentinel::application::system::manage_quarantine_service::ManageQuarantineService::new(
            quarantine_repo,
            // Le delai avant expulsion, le rappel et l'expulsion elle-meme sont
            // des reglages du SERVEUR : le use case les lit ici plutot que de
            // recevoir une duree decidee par l'appelant.
            bot_config_repo.clone(),
        ),
    );

    // Lockdown de securite : repo Postgres (SQL security_lockdown_active) + use
    // case (calcul de l'expiration). Le handler ne fait que parse/RBAC/map.
    let lockdown_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::system::lockdown_repository::LockdownRepository,
    > = Arc::new(
        crate::sentinel::adapters::outbound::postgres::system::lockdown_repository::PgLockdownRepository::new(
            pg_pool.clone(),
        ),
    );
    let lockdown_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::system::manage_lockdown::ManageLockdownUseCase,
    > = Arc::new(
        platform_core::sentinel::application::system::manage_lockdown_service::ManageLockdownService::new(
            lockdown_repo,
        ),
    );

    // Slowmode de securite manuel : repo Postgres (SQL security_slowmode_active) +
    // use case (calcul de l'expiration). Le handler ne fait que parse/RBAC/map.
    // Distinct de l'automod adaptatif (moderation).
    let slowmode_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::system::slowmode_repository::SlowmodeRepository,
    > = Arc::new(
        crate::sentinel::adapters::outbound::postgres::system::slowmode_repository::PgSlowmodeRepository::new(
            pg_pool.clone(),
        ),
    );
    let slowmode_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase,
    > = Arc::new(
        platform_core::sentinel::application::system::manage_slowmode_service::ManageSlowmodeService::new(
            slowmode_repo,
        ),
    );

    // Persistance fire-and-forget des bots (streaks, etc.) : repo Postgres
    // (SQL user_levels) + use case pass-through. Le handler ne fait que
    // parser/valider/mapper.
    let bot_persistence_repo: Arc<dyn platform_core::sentinel::ports::outbound::system::bot_persistence_repository::BotPersistenceRepository> =
        Arc::new(crate::sentinel::adapters::outbound::postgres::system::bot_persistence_repository::PgBotPersistenceRepository::new(pg_pool.clone()));
    let bot_persistence_uc: Arc<dyn platform_core::sentinel::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase> =
        Arc::new(platform_core::sentinel::application::system::manage_bot_persistence_service::ManageBotPersistenceService::new(
            bot_persistence_repo,
        ));
    // Daemon Docker de l'hote, via `docker-agent`. Ce processus ne monte plus
    // `/var/run/docker.sock` : le socket equivaut a un acces root, et il n'a
    // rien a faire dans l'API qui sert aussi l'OAuth et la moderation.

    // Sync Discord <-> Web (Phase 1 — cf. SYNC_DISCORD_WEB_DESIGN.md).
    // Repo outbound + use case inbound : on injecte uniquement le use
    // case dans AppState pour respecter l'archi hexagonale (handlers
    // HTTP/gRPC ne touchent jamais les repos directement).
    let discord_action_message_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository,
    > = Arc::new(
        crate::sentinel::adapters::outbound::postgres::audit::discord_action_message_repository::PgDiscordActionMessageRepository::new(
            pg_pool.clone(),
        ),
    );
    let discord_action_messages_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase,
    > = Arc::new(platform_core::sentinel::application::audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService::new(
        discord_action_message_repo,
    ));

    let watched_users_uc = Arc::new(ManageWatchedUsersService::new(
        watched_user_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        security_uc.clone(),
    ));

    // ── State ──
    let modstats_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::audit::modstats_repository::ModstatsRepository,
    > = Arc::new(PgModstatsRepository::new(pg_pool.clone()));
    let modstats_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::moderation::read_modstats::ReadModstatsUseCase,
    > = Arc::new(
        platform_core::sentinel::application::moderation::read_modstats_service::ReadModstatsService::new(
            modstats_repo.clone(),
        ),
    );

    // ── Ban en sursis (moderation) ──
    let sursis_repo: Arc<
        dyn platform_core::sentinel::ports::outbound::moderation::sursis_repository::SursisRepository,
    > = Arc::new(
        crate::sentinel::adapters::outbound::postgres::moderation::sursis_repository::PgSursisRepository::new(
            pg_pool.clone(),
        ),
    );
    let sursis_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::moderation::manage_sursis::ManageSursisUseCase,
    > = Arc::new(
        platform_core::sentinel::application::moderation::manage_sursis_service::ManageSursisService::new(
            sursis_repo,
        ),
    );

    let strikes_uc: Arc<
        dyn platform_core::sentinel::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase,
    > = Arc::new(
        platform_core::sentinel::application::moderation::manage_strikes_service::ManageStrikesService::new(
            strike_repo.clone(),
        ),
    );

    include!("app_state/assembly.rs")
}
