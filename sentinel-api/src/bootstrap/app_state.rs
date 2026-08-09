//! Assemblage complet de l'AppState : tous les repos + services (DI).

use std::sync::Arc;

use crate::adapters::inbound::http::state::AppState;
use crate::adapters::outbound::batching::audit_log_batcher::BatchedPgAuditLogRepository;
use crate::adapters::outbound::batching::batch_writer::BatchWriterConfig;
use crate::adapters::outbound::batching::log_batcher::BatchedPgLogRepository;
use crate::adapters::outbound::discord_api::DiscordApiService;
use crate::adapters::outbound::job_client::JobClient;
use crate::adapters::outbound::postgres::audit::analytics_repository::PgAnalyticsRepository;
use crate::adapters::outbound::postgres::audit::modstats_repository::PgModstatsRepository;
use crate::adapters::outbound::postgres::audit::security_event_repository::PgSecurityEventRepository;
use crate::adapters::outbound::postgres::audit::stats_repository::PgStatsRepository;
use crate::adapters::outbound::postgres::audit::user_activity_repository::PgUserActivityRepository;
use crate::adapters::outbound::postgres::audit::watched_user_repository::PgWatchedUserRepository;
use crate::adapters::outbound::postgres::community::daily_activity_repository::PgDailyActivityRepository;
use crate::adapters::outbound::postgres::community::discord_role_repository::PgDiscordRoleRepository;
use crate::adapters::outbound::postgres::community::level_repository::PgLevelRepository;
use crate::adapters::outbound::postgres::community::member_repository::PgMemberRepository;
use crate::adapters::outbound::postgres::community::role_panel_repository::PgRolePanelRepository;
use crate::adapters::outbound::postgres::community::sponsorship_repository::PgSponsorshipRepository;
use crate::adapters::outbound::postgres::community::temp_role_repository::PgTempRoleRepository;
use crate::adapters::outbound::postgres::community::voice_channel_repository::PgVoiceChannelRepository;
use crate::adapters::outbound::postgres::community::welcome_config_repository::PgWelcomeConfigRepository;
use crate::adapters::outbound::postgres::moderation::evidence_repository::PgEvidenceRepository;
use crate::adapters::outbound::postgres::moderation::infraction_repository::PgInfractionRepository;
use crate::adapters::outbound::postgres::moderation::moderation_repository::PgModerationRepository;
use crate::adapters::outbound::postgres::moderation::notes_repository::PgNotesRepository;
use crate::adapters::outbound::postgres::moderation::pending_action_repository::PgPendingActionRepository;
use crate::adapters::outbound::postgres::moderation::reminder_repository::PgReminderRepository;
use crate::adapters::outbound::postgres::moderation::review_repository::PgReviewRepository;
use crate::adapters::outbound::postgres::moderation::rule_repository::PgRuleRepository;
use crate::adapters::outbound::postgres::moderation::strike_repository::PgStrikeRepository;
use crate::adapters::outbound::postgres::system::bot_config_repository::PgBotConfigRepository;
use crate::adapters::outbound::postgres::system::guild_repository::PgGuildRepository;
use crate::adapters::outbound::postgres::system::ticket_repository::PgTicketRepository;
use crate::adapters::outbound::redis_cache::RedisCache;
use crate::config::AppConfig;
use sentinel_core::application::ai::analyze_image_service::AnalyzeImageService;
use sentinel_core::application::ai::analyze_message_service::AnalyzeMessageService;
use sentinel_core::application::audit::manage_audit_logs_service::ManageAuditLogsService;
use sentinel_core::application::audit::manage_security_service::ManageSecurityService;
use sentinel_core::application::audit::manage_stats_service::ManageStatsService;
use sentinel_core::application::audit::manage_watched_users_service::ManageWatchedUsersService;
use sentinel_core::application::community::manage_levels_service::ManageLevelsService;
use sentinel_core::application::community::manage_members_service::ManageMembersService;
use sentinel_core::application::community::manage_role_panels_service::ManageRolePanelsService;
use sentinel_core::application::community::voice_channels::ManageVoiceChannelsService;
use sentinel_core::application::moderation::manage_infractions_service::ManageInfractionsService;
use sentinel_core::application::moderation::manage_moderation_service::ManageModerationService;
use sentinel_core::application::moderation::manage_notes_service::ManageNotesService;
use sentinel_core::application::moderation::manage_reminders_service::ManageRemindersService;
use sentinel_core::application::moderation::manage_rules_service::ManageRulesService;
use sentinel_core::application::moderation::manage_strikes_service::ManageStrikesService;
use sentinel_core::application::system::export_service::ExportService;
use sentinel_core::application::system::manage_tickets_service::ManageTicketsService;

/// Construit l'etat complet de l'application (tous les repos + services).
/// Consomme le pool et le client Redis (via clones).
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
    let voice_channel_repo = Arc::new(PgVoiceChannelRepository::new(pg_pool.clone()));
    let age_ban_repo = Arc::new(
        crate::adapters::outbound::postgres::community::age_ban_repository::PgAgeBanRepository::new(
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
    // Use case lecture/purge des logs systeme — reutilise le meme repo batche.
    let system_logs_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_system_logs::ManageSystemLogsUseCase> =
        Arc::new(sentinel_core::application::system::manage_system_logs_service::ManageSystemLogsService::new(
            log_repo.clone(),
        ));
    let notes_repo = Arc::new(PgNotesRepository::new(pg_pool.clone()));
    let reminder_repo = Arc::new(PgReminderRepository::new(pg_pool.clone()));
    let strike_repo = Arc::new(PgStrikeRepository::new(pg_pool.clone()));
    let cache = Arc::new(
        RedisCache::new(redis_client.clone())
            .await
            .expect("Impossible d'etablir la connexion Redis pour le cache"),
    );

    // ── Event broadcaster (Redis pub/sub → gateway WebSocket) ──
    let broadcaster = crate::bootstrap::build_broadcaster(redis_client.clone());

    // ── Inference ONNX ──
    let (inference, tokenizer, inference_limiter) = crate::bootstrap::build_inference();

    // Discord API (un seul client partage).
    let discord_api: Arc<dyn crate::adapters::outbound::discord_api::DiscordApi> =
        Arc::new(DiscordApiService::new(config.discord_bot_token.clone()));

    // ── Services applicatifs ──
    // Buffer in-memory partage (tension de salon). Pas de persistance :
    // reset au restart bot, c'est OK car seulement les N derniers messages.
    let channel_tension_buffer = Arc::new(
        sentinel_core::domain::services::moderation::channel_tension::ChannelTensionBuffer::new(),
    );

    let deepseek_service = Arc::new(
        crate::adapters::outbound::deepseek_moderation_service::DeepSeekModerationAdapter::from_env(
        ),
    );

    let analyze_uc = Arc::new(
        AnalyzeMessageService::new(
            rule_repo.clone(),
            infraction_repo.clone(),
            cache.clone(),
            bot_config_repo.clone(),
            inference_limiter.clone(),
        )
        .with_deepseek(deepseek_service)
        .with_text_inference(inference.clone(), tokenizer)
        .with_channel_tension(channel_tension_buffer.clone()),
    );
    let analyze_image_uc = Arc::new(AnalyzeImageService::new(
        inference.clone(),
        rule_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        bot_config_repo.clone(),
        inference_limiter.clone(),
    ));
    // Dataset IA : repo Postgres (SQL ai_dataset_messages) + use case (bornage
    // des filtres, validation des ids). Le handler ne fait que RBAC + map.
    let dataset_repo = Arc::new(
        crate::adapters::outbound::postgres::ai::dataset_repository::PgDatasetRepository::new(
            pg_pool.clone(),
        ),
    );
    let dataset_uc: Arc<
        dyn sentinel_core::ports::inbound::ai::manage_dataset::ManageDatasetUseCase,
    > = Arc::new(
        sentinel_core::application::ai::manage_dataset_service::ManageDatasetService::new(
            dataset_repo,
        ),
    );

    // File de jobs IA : repo Postgres (SQL ai_jobs) + use case (validation
    // job_type/guild_id). Le handler ne fait que parse/map.
    let ai_job_repo = Arc::new(
        crate::adapters::outbound::postgres::ai::ai_job_repository::PgAiJobRepository::new(
            pg_pool.clone(),
        ),
    );
    let ai_jobs_uc: Arc<
        dyn sentinel_core::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase,
    > = Arc::new(
        sentinel_core::application::ai::manage_ai_jobs_service::ManageAiJobsService::new(
            ai_job_repo,
        ),
    );

    let rules_uc = Arc::new(ManageRulesService::new(rule_repo.clone(), cache.clone()));
    let infractions_uc = Arc::new(ManageInfractionsService::new(infraction_repo.clone()));
    let tickets_uc = Arc::new(ManageTicketsService::new(
        ticket_repo.clone(),
        cache.clone(),
    ));
    // Phase 5C — Batch writes : idem que log_repo, pour les audit events.
    // Phase 1 dual-write : creation deplacee plus tot pour pouvoir injecter
    // audit_logs_uc dans security_uc et moderation_uc.
    let audit_log_repo = Arc::new(BatchedPgAuditLogRepository::new(
        pg_pool.clone(),
        BatchWriterConfig::default(),
    ));
    let audit_logs_uc = Arc::new(ManageAuditLogsService::new(audit_log_repo));

    // Planning communautaire : evenements et campagnes de jeu.
    let events_uc = Arc::new(
        sentinel_core::application::community::manage_events_service::ManageEventsService::new(
            Arc::new(
                crate::adapters::outbound::postgres::community::event_repository::PgEventRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    // Vie de la communaute : annonces de recherche de joueurs, sondages,
    // membre du mois, nouvelles du site. Quatre concepts distincts mais un
    // seul public : la page de l'espace membre.
    let lfg_uc = Arc::new(
        sentinel_core::application::community::manage_lfg_service::ManageLfgService::new(Arc::new(
            crate::adapters::outbound::postgres::community::lfg_repository::PgLfgRepository::new(
                pg_pool.clone(),
            ),
        )),
    );
    let polls_uc = Arc::new(
        sentinel_core::application::community::manage_polls_service::ManagePollsService::new(
            Arc::new(
                crate::adapters::outbound::postgres::community::poll_repository::PgPollRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );
    let spotlight_uc = Arc::new(
        sentinel_core::application::community::manage_spotlight_service::ManageSpotlightService::new(
            Arc::new(
                crate::adapters::outbound::postgres::community::spotlight_repository::PgSpotlightRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );
    let news_uc = Arc::new(
        sentinel_core::application::community::manage_news_service::ManageNewsService::new(
            Arc::new(
                crate::adapters::outbound::postgres::community::news_repository::PgNewsRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    // Presence en direct. Le bot publie, l'API lit : elle n'a aucun moyen de
    // savoir qui est en vocal, et fabriquer cette donnee ici serait mentir.
    let presence_uc = Arc::new(
        sentinel_core::application::community::read_presence_service::ReadPresenceService::new(
            Arc::new(
                crate::adapters::outbound::redis_presence::RedisPresenceRepository::new(
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
        crate::adapters::outbound::audit::in_memory_anomaly_counter::InMemoryAnomalyCounter::new(
            anomaly_max_buffer,
            anomaly_eviction_target,
        ),
    );
    let detect_anomaly_uc = Arc::new(
        sentinel_core::application::audit::detect_moderation_anomaly_service::DetectModerationAnomalyService::new(
            anomaly_counter,
        ),
    );

    // Rapport hebdomadaire agrege server-side : comptage postgres par event_type
    // sur 7 jours (remonte de l'ancien WeeklyTracker RAM du bot).
    let weekly_report_uc = Arc::new(
        sentinel_core::application::audit::get_weekly_report_service::GetWeeklyReportService::new(
            Arc::new(
                crate::adapters::outbound::postgres::audit::audit_event_counter::PgAuditEventCounter::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    );

    let user_activity_repo: Arc<
        dyn sentinel_core::ports::outbound::audit::user_activity_repository::UserActivityRepository,
    > = Arc::new(PgUserActivityRepository::new(pg_pool.clone()));
    let welcome_config_repo: Arc<
        dyn sentinel_core::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository,
    > = Arc::new(PgWelcomeConfigRepository::new(pg_pool.clone()));
    // Use case Welcome (Phase 3) — handlers HTTP/gRPC passent par ce port
    // inbound, jamais par le repo direct.
    let welcome_config_uc: Arc<dyn sentinel_core::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase> =
        Arc::new(sentinel_core::application::community::manage_welcome_config_service::ManageWelcomeConfigService::new(
            welcome_config_repo.clone(),
        ));
    // Verification d'age : DECISION server-side (le bot n'execute que l'action
    // Discord). Lit la config welcome du serveur via le meme repo.
    let age_check_uc: Arc<dyn sentinel_core::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase> =
        Arc::new(sentinel_core::application::community::evaluate_age_declaration_service::EvaluateAgeDeclarationService::new(
            welcome_config_repo.clone(),
        ));
    // Automod reviews (sync Discord <-> web).
    let automod_review_repo: Arc<dyn sentinel_core::ports::outbound::moderation::automod_review_repository::AutomodReviewRepository> = Arc::new(
        crate::adapters::outbound::postgres::moderation::automod_review_repository::PgAutomodReviewRepository::new(pg_pool.clone()),
    );
    let automod_adaptive_slowmode_repo: Arc<dyn sentinel_core::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository> = Arc::new(
        crate::adapters::outbound::postgres::moderation::adaptive_slowmode_repository::PgAdaptiveSlowmodeRepository::new(pg_pool.clone()),
    );
    let automod_reviews_uc: Arc<dyn sentinel_core::ports::inbound::moderation::manage_automod_reviews::ManageAutomodReviewsUseCase> =
        Arc::new(sentinel_core::application::moderation::manage_automod_reviews_service::ManageAutomodReviewsService::new(
            automod_review_repo.clone(),
        ));

    // Reset complet d'un serveur (factory reset, owner-only).
    let reset_guild_uc: Arc<dyn sentinel_core::ports::inbound::system::reset_guild::ResetGuildUseCase> =
        Arc::new(sentinel_core::application::system::reset_guild_service::ResetGuildService::new(Arc::new(
            crate::adapters::outbound::postgres::system::guild_reset_repository::PgGuildResetRepository::new(pg_pool.clone()),
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
                dyn sentinel_core::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase,
            >),
    );
    // Note : la creation de moderation_uc est differee plus bas pour pouvoir
    // injecter strikes_uc via with_strikes_uc (log_action_with_strike).
    let service_registry: Arc<
        dyn sentinel_core::ports::outbound::system::service_registry::ServiceRegistry,
    > = Arc::new(
        crate::adapters::outbound::redis_service_registry::RedisServiceRegistry::new(
            redis_client.clone(),
        ),
    );
    let stats_uc = Arc::new(ManageStatsService::new(
        stats_repo.clone(),
        infraction_repo.clone(),
        cache.clone(),
        service_registry,
    ));
    let voice_channels_uc = Arc::new(ManageVoiceChannelsService::new(
        voice_channel_repo.clone(),
        cache.clone(),
        bot_config_repo.clone(),
    ));
    let role_panel_repo = Arc::new(PgRolePanelRepository::new(pg_pool.clone()));
    let role_panels_uc = Arc::new(ManageRolePanelsService::new(role_panel_repo));
    let analytics_repo = Arc::new(PgAnalyticsRepository::new(pg_pool.clone()));
    let daily_activity_repo = Arc::new(PgDailyActivityRepository::new(pg_pool.clone()));
    let level_repo = Arc::new(PgLevelRepository::new(pg_pool.clone()));
    let levels_uc = Arc::new(ManageLevelsService::new(
        level_repo,
        bot_config_repo.clone(),
    ));
    let announcement_repo = Arc::new(crate::adapters::outbound::postgres::community::announcement_repository::PgAnnouncementRepository::new(pg_pool.clone()));
    let announcements_uc: Arc<dyn sentinel_core::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase> = Arc::new(sentinel_core::application::community::manage_announcements_service::ManageAnnouncementsService::new(announcement_repo, bot_config_repo.clone()));
    let embed_repo = Arc::new(
        crate::adapters::outbound::postgres::community::embed_repository::PgEmbedRepository::new(
            pg_pool.clone(),
        ),
    );
    let embeds_uc: Arc<
        dyn sentinel_core::ports::inbound::community::manage_embeds::ManageEmbedsUseCase,
    > = Arc::new(
        sentinel_core::application::community::manage_embeds_service::ManageEmbedsService::new(
            embed_repo,
        ),
    );
    let idea_repo = Arc::new(
        crate::adapters::outbound::postgres::community::idea_repository::PgIdeaRepository::new(
            pg_pool.clone(),
        ),
    );
    let ideas_uc: Arc<
        dyn sentinel_core::ports::inbound::community::manage_ideas::ManageIdeasUseCase,
    > = Arc::new(
        sentinel_core::application::community::manage_ideas_service::ManageIdeasService::new(
            idea_repo,
        ),
    );
    let confession_repo = Arc::new(crate::adapters::outbound::postgres::community::confession_repository::PgConfessionRepository::new(pg_pool.clone()));
    let confessions_uc: Arc<
        dyn sentinel_core::ports::inbound::community::manage_confessions::ManageConfessionsUseCase,
    > = Arc::new(
        sentinel_core::application::community::manage_confessions_service::ManageConfessionsService::new(
            confession_repo,
        ),
    );
    let notes_uc = Arc::new(ManageNotesService::new(notes_repo));
    let reminders_uc = Arc::new(ManageRemindersService::new(reminder_repo));
    let strikes_uc = Arc::new(ManageStrikesService::new(strike_repo.clone()));
    // Copilote de moderation (lecture seule) : reutilise le use case strikes
    // (ladder d'escalade) + un port focalise pour l'historique & la
    // jurisprudence automod (anti-ancrage : exclut les reviews 'voting').
    let moderation_copilot_repo: Arc<
        dyn sentinel_core::ports::outbound::moderation::moderation_copilot_repository::ModerationCopilotRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::moderation::moderation_copilot_repository::PgModerationCopilotRepository::new(pg_pool.clone()),
    );
    let moderation_copilot_uc: Arc<
        dyn sentinel_core::ports::inbound::moderation::moderation_copilot::ModerationCopilotUseCase,
    > = Arc::new(
        sentinel_core::application::moderation::manage_moderation_copilot_service::ManageModerationCopilotService::new(
            strikes_uc.clone()
                as Arc<dyn sentinel_core::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase>,
            moderation_copilot_repo,
        ),
    );
    // Evaluation server-side du risque de cible (garde-fou UX confirmation) :
    // lit le seuil `risk_recent_account_days` (moderation-bot, defaut 7j) et
    // applique la politique. Le bot ne fournit que les faits Discord.
    let assess_target_risk_uc: Arc<
        dyn sentinel_core::ports::inbound::moderation::assess_target_risk::AssessTargetRiskUseCase,
    > = Arc::new(
        sentinel_core::application::moderation::assess_target_risk_service::AssessTargetRiskService::new(
            bot_config_repo.clone(),
        ),
    );
    let moderation_uc = Arc::new(
        ManageModerationService::new(moderation_repo.clone(), strike_repo.clone(), cache.clone())
            .with_strikes_uc(strikes_uc.clone()
                as Arc<dyn sentinel_core::ports::inbound::moderation::manage_strikes::ManageStrikesUseCase>)
            .with_audit_logs_uc(audit_logs_uc.clone()
                as Arc<
                    dyn sentinel_core::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase,
                >),
    );
    let member_repo = Arc::new(PgMemberRepository::new(pg_pool.clone()));
    let discord_role_repo = Arc::new(PgDiscordRoleRepository::new(pg_pool.clone()));

    // Eligibilite Community : decisions server-side (prerequis de role +
    // validation de parrainage). Lit la config via bot_config_repo ; regles
    // pures dans le domaine. Le bot ne fournit que les donnees Discord.
    let eligibility_uc: Arc<
        dyn sentinel_core::ports::inbound::community::check_eligibility::CheckEligibilityUseCase,
    > = Arc::new(
        sentinel_core::application::community::check_eligibility_service::CheckEligibilityService::new(
            bot_config_repo.clone(),
        ),
    );

    // Classement mensuel : repo Postgres (deltas d'XP + baselines) + use case
    // (gates de publication, assemblage des tops, pose des baselines). Le
    // handler HTTP ne fait que RBAC + envoi Discord.
    let monthly_ranking_repo = Arc::new(
        crate::adapters::outbound::postgres::community::monthly_ranking_repository::PgMonthlyRankingRepository::new(
            pg_pool.clone(),
        ),
    );
    let monthly_ranking_uc: Arc<
        dyn sentinel_core::ports::inbound::community::manage_monthly_ranking::ManageMonthlyRankingUseCase,
    > = Arc::new(
        sentinel_core::application::community::manage_monthly_ranking_service::ManageMonthlyRankingService::new(
            bot_config_repo.clone(),
            monthly_ranking_repo,
        ),
    );

    // Snapshots analytics : repo Postgres (SQL des jobs) + use case (config par
    // guild, deltas de baseline, filtres de publication). Les handlers HTTP ne
    // font que declencher/RBAC/poster.
    let snapshot_repo = Arc::new(
        crate::adapters::outbound::postgres::audit::snapshot_repository::PgSnapshotRepository::new(
            pg_pool.clone(),
        ),
    );
    let snapshots_uc: Arc<
        dyn sentinel_core::ports::inbound::audit::manage_snapshots::ManageSnapshotsUseCase,
    > = Arc::new(
        sentinel_core::application::audit::manage_snapshots_service::ManageSnapshotsService::new(
            bot_config_repo.clone(),
            snapshot_repo,
            analytics_repo.clone(),
        ),
    );

    // Sauvegarde / restauration de serveur (guild_backup) : repo + use case.
    let snapshot_repo: Arc<dyn sentinel_core::ports::outbound::guild_backup::snapshot_repository::SnapshotRepository> =
        Arc::new(
            crate::adapters::outbound::postgres::guild_backup::snapshot_repository::PgSnapshotRepository::new(
                pg_pool.clone(),
            ),
        );
    let guild_snapshots_uc: Arc<dyn sentinel_core::ports::inbound::guild_backup::manage_snapshots::ManageGuildSnapshotsUseCase> =
        Arc::new(
            sentinel_core::application::guild_backup::manage_snapshots_service::ManageGuildSnapshotsService::new(
                snapshot_repo.clone(),
            ),
        );
    // Re-attribution des roles aux membres de retour (pending_role_grants).
    let pending_role_grant_repo: Arc<dyn sentinel_core::ports::outbound::guild_backup::pending_role_grant_repository::PendingRoleGrantRepository> =
        Arc::new(
            crate::adapters::outbound::postgres::guild_backup::pending_role_grant_repository::PgPendingRoleGrantRepository::new(
                pg_pool.clone(),
            ),
        );
    let pending_role_grants_uc: Arc<dyn sentinel_core::ports::inbound::guild_backup::manage_pending_role_grants::ManagePendingRoleGrantsUseCase> =
        Arc::new(
            sentinel_core::application::guild_backup::manage_pending_role_grants_service::ManagePendingRoleGrantsService::new(
                pending_role_grant_repo.clone(),
            ),
        );

    // Bans IP (panel securite) : repo DB + file-shim host + reader fail2ban.
    let ip_ban_repo: Arc<
        dyn sentinel_core::ports::outbound::system::ip_ban_repository::IpBanRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::system::ip_ban_repository::PgIpBanRepository::new(
            pg_pool.clone(),
        ),
    );
    let host_ban_queue: Arc<
        dyn sentinel_core::ports::outbound::system::host_ban_queue::HostBanQueue,
    > = Arc::new(crate::adapters::outbound::host_security::ban_queue::FileBanQueue::new());
    let fail2ban_reader: Arc<
        dyn sentinel_core::ports::outbound::system::host_ban_queue::Fail2banStatusReader,
    > = Arc::new(crate::adapters::outbound::host_security::fail2ban::Fail2banFileReader::new());
    let ip_bans_uc: Arc<
        dyn sentinel_core::ports::inbound::system::manage_ip_bans::ManageIpBansUseCase,
    > = Arc::new(
        sentinel_core::application::system::manage_ip_bans_service::ManageIpBansService::new(
            ip_ban_repo,
            host_ban_queue,
            fail2ban_reader,
        ),
    );

    // Sondes de securite host (JSON cron) : reader fichier + use case pass-through.
    let host_probe_reader: Arc<
        dyn sentinel_core::ports::outbound::system::host_probe_reader::HostProbeReader,
    > = Arc::new(
        crate::adapters::outbound::host_security::probe_reader::FileHostProbeReader::new(),
    );
    let host_probe_uc: Arc<
        dyn sentinel_core::ports::inbound::system::read_host_probe::ReadHostProbeUseCase,
    > = Arc::new(
        sentinel_core::application::system::read_host_probe_service::ReadHostProbeService::new(
            host_probe_reader,
        ),
    );

    // Analyse des logs securite (top IPs, echecs d'auth, trafic).
    let security_log_repo: Arc<dyn sentinel_core::ports::outbound::system::security_log_repository::SecurityLogRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::security_log_repository::PgSecurityLogRepository::new(pg_pool.clone()));
    let security_logs_uc: Arc<dyn sentinel_core::ports::inbound::system::read_security_logs::ReadSecurityLogsUseCase> =
        Arc::new(sentinel_core::application::system::read_security_logs_service::ReadSecurityLogsService::new(security_log_repo));

    // Audit & maintenance securite (journal d'audit, logins, purge des logs).
    let security_audit_repo: Arc<dyn sentinel_core::ports::outbound::system::security_audit_repository::SecurityAuditRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::security_audit_repository::PgSecurityAuditRepository::new(pg_pool.clone()));
    let security_audit_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_security_audit::ManageSecurityAuditUseCase> =
        Arc::new(sentinel_core::application::system::manage_security_audit_service::ManageSecurityAuditService::new(security_audit_repo));

    // OAuth Discord web : repo Postgres (sessions + logins) + use case. Le SQL
    // vit dans l'adapter ; l'echange HTTP avec Discord + CSRF/cookies restent
    // au handler (concern HTTP).
    let oauth_session_repo: Arc<dyn sentinel_core::ports::outbound::system::oauth_session_repository::OAuthSessionRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::oauth_session_repository::PgOAuthSessionRepository::new(pg_pool.clone()));
    let oauth_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_oauth::ManageOAuthUseCase> =
        Arc::new(
            sentinel_core::application::system::manage_oauth_service::ManageOAuthService::new(
                oauth_session_repo,
            ),
        );

    // Quarantaine de securite : repo Postgres (SQL security_quarantine_pending) +
    // use case (calcul du delai avant kick). Le handler ne fait que parse/RBAC/map.
    let quarantine_repo: Arc<dyn sentinel_core::ports::outbound::system::quarantine_repository::QuarantineRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::quarantine_repository::PgQuarantineRepository::new(pg_pool.clone()));
    let quarantine_uc: Arc<
        dyn sentinel_core::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase,
    > = Arc::new(
        sentinel_core::application::system::manage_quarantine_service::ManageQuarantineService::new(
            quarantine_repo,
        ),
    );

    // Lockdown de securite : repo Postgres (SQL security_lockdown_active) + use
    // case (calcul de l'expiration). Le handler ne fait que parse/RBAC/map.
    let lockdown_repo: Arc<
        dyn sentinel_core::ports::outbound::system::lockdown_repository::LockdownRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::system::lockdown_repository::PgLockdownRepository::new(
            pg_pool.clone(),
        ),
    );
    let lockdown_uc: Arc<
        dyn sentinel_core::ports::inbound::system::manage_lockdown::ManageLockdownUseCase,
    > = Arc::new(
        sentinel_core::application::system::manage_lockdown_service::ManageLockdownService::new(
            lockdown_repo,
        ),
    );

    // Slowmode de securite manuel : repo Postgres (SQL security_slowmode_active) +
    // use case (calcul de l'expiration). Le handler ne fait que parse/RBAC/map.
    // Distinct de l'automod adaptatif (moderation).
    let slowmode_repo: Arc<
        dyn sentinel_core::ports::outbound::system::slowmode_repository::SlowmodeRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::system::slowmode_repository::PgSlowmodeRepository::new(
            pg_pool.clone(),
        ),
    );
    let slowmode_uc: Arc<
        dyn sentinel_core::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase,
    > = Arc::new(
        sentinel_core::application::system::manage_slowmode_service::ManageSlowmodeService::new(
            slowmode_repo,
        ),
    );

    // Règles d'alerte de supervision : repo Postgres (SQL alert_rules) + use
    // case (invariants sévérité/cooldown). Le handler ne fait que RBAC/mapper.
    let alert_rules_repo: Arc<dyn sentinel_core::ports::outbound::system::alert_rule_repository::AlertRuleRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::alert_rule_repository::PgAlertRuleRepository::new(pg_pool.clone()));
    let alert_rules_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_alert_rules::ManageAlertRulesUseCase> =
        Arc::new(sentinel_core::application::system::manage_alert_rules_service::ManageAlertRulesService::new(
            alert_rules_repo,
        ));

    // Persistance fire-and-forget des bots (streaks, etc.) : repo Postgres
    // (SQL user_levels) + use case pass-through. Le handler ne fait que
    // parser/valider/mapper.
    let bot_persistence_repo: Arc<dyn sentinel_core::ports::outbound::system::bot_persistence_repository::BotPersistenceRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::bot_persistence_repository::PgBotPersistenceRepository::new(pg_pool.clone()));
    let bot_persistence_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_bot_persistence::ManageBotPersistenceUseCase> =
        Arc::new(sentinel_core::application::system::manage_bot_persistence_service::ManageBotPersistenceService::new(
            bot_persistence_repo,
        ));

    // Audit serveur (server_events) : repo Postgres + use case (bornage des
    // filtres de lecture). Le handler ne fait que parse/RBAC/map.
    let server_event_repo: Arc<dyn sentinel_core::ports::outbound::system::server_event_repository::ServerEventRepository> =
        Arc::new(crate::adapters::outbound::postgres::system::server_event_repository::PgServerEventRepository::new(pg_pool.clone()));
    let server_events_uc: Arc<dyn sentinel_core::ports::inbound::system::manage_server_events::ManageServerEventsUseCase> =
        Arc::new(sentinel_core::application::system::manage_server_events_service::ManageServerEventsService::new(
            server_event_repo,
        ));

    // Daemon Docker de l'hote : client bollard derriere le port DockerHost.
    let docker_host: Arc<dyn sentinel_core::ports::outbound::system::docker_host::DockerHost> =
        Arc::new(crate::adapters::outbound::system::docker_host::BollardDockerHost);

    // Cert TLS + GeoIP (infra externe : fichier/openssl + http ip-api).
    let tls_cert_reader: Arc<
        dyn sentinel_core::ports::outbound::system::tls_cert_reader::TlsCertReader,
    > = Arc::new(crate::adapters::outbound::host_security::tls_cert::FileTlsCertReader::new());
    let tls_cert_uc: Arc<
        dyn sentinel_core::ports::inbound::system::read_tls_cert::ReadTlsCertUseCase,
    > = Arc::new(
        sentinel_core::application::system::read_tls_cert_service::ReadTlsCertService::new(
            tls_cert_reader,
        ),
    );
    let geoip_lookup: Arc<dyn sentinel_core::ports::outbound::system::geoip_lookup::GeoIpLookup> =
        Arc::new(crate::adapters::outbound::geoip::IpApiGeoIpLookup::new());
    let geoip_uc: Arc<dyn sentinel_core::ports::inbound::system::lookup_geoip::LookupGeoIpUseCase> =
        Arc::new(
            sentinel_core::application::system::lookup_geoip_service::LookupGeoIpService::new(
                geoip_lookup,
            ),
        );

    // Sync Discord <-> Web (Phase 1 — cf. SYNC_DISCORD_WEB_DESIGN.md).
    // Repo outbound + use case inbound : on injecte uniquement le use
    // case dans AppState pour respecter l'archi hexagonale (handlers
    // HTTP/gRPC ne touchent jamais les repos directement).
    let discord_action_message_repo: Arc<
        dyn sentinel_core::ports::outbound::audit::discord_action_message_repository::DiscordActionMessageRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::audit::discord_action_message_repository::PgDiscordActionMessageRepository::new(
            pg_pool.clone(),
        ),
    );
    let discord_action_messages_uc: Arc<
        dyn sentinel_core::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase,
    > = Arc::new(sentinel_core::application::audit::manage_discord_action_messages_service::ManageDiscordActionMessagesService::new(
        discord_action_message_repo,
    ));

    let watched_users_uc = Arc::new(ManageWatchedUsersService::new(
        watched_user_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        security_uc.clone(),
        notes_uc.clone(),
    ));

    let members_uc = Arc::new(ManageMembersService::new(
        member_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        stats_uc.clone(),
    ));

    // ── Discord API service : instance deja creee plus haut.
    // On re-declare ici pour garder la variable accessible dans la suite du
    // bootstrap (AppState.discord_api).

    // ── Job client (queue Redis → worker) ──
    let queue_key =
        std::env::var("REDIS_QUEUE_KEY").unwrap_or_else(|_| "sentinel:jobs".to_string());
    let job_client = JobClient::new(redis_client.clone(), queue_key);

    // ── State ──
    let modstats_repo: Arc<
        dyn sentinel_core::ports::outbound::audit::modstats_repository::ModstatsRepository,
    > = Arc::new(PgModstatsRepository::new(pg_pool.clone()));
    let modstats_uc: Arc<
        dyn sentinel_core::ports::inbound::moderation::read_modstats::ReadModstatsUseCase,
    > = Arc::new(
        sentinel_core::application::moderation::read_modstats_service::ReadModstatsService::new(
            modstats_repo.clone(),
        ),
    );

    // ── Ban en sursis (moderation) ──
    let sursis_repo: Arc<
        dyn sentinel_core::ports::outbound::moderation::sursis_repository::SursisRepository,
    > = Arc::new(
        crate::adapters::outbound::postgres::moderation::sursis_repository::PgSursisRepository::new(
            pg_pool.clone(),
        ),
    );
    let sursis_uc: Arc<
        dyn sentinel_core::ports::inbound::moderation::manage_sursis::ManageSursisUseCase,
    > = Arc::new(
        sentinel_core::application::moderation::manage_sursis_service::ManageSursisService::new(
            sursis_repo,
        ),
    );

    // ── Sous-etats par domaine ──
    //
    // Construits AVANT le `AppState` plat pour que ce dernier puisse cloner
    // depuis eux : une seule source par port, pas deux instanciations qui
    // divergeraient silencieusement. Cf. `bootstrap::state` pour le pourquoi
    // de la coexistence des deux formes pendant la migration.
    let ai = crate::bootstrap::state::AiState {
        analyze_uc: analyze_uc.clone(),
        analyze_image_uc: analyze_image_uc.clone(),
        dataset_uc: dataset_uc.clone(),
        ai_jobs_uc: ai_jobs_uc.clone(),
        inference: inference.clone(),
        broadcaster: broadcaster.clone(),
    };

    let moderation = crate::bootstrap::state::ModerationState {
        rules_uc: rules_uc.clone(),
        infractions_uc: infractions_uc.clone(),
        moderation_uc: moderation_uc.clone(),
        modstats_uc: modstats_uc.clone(),
        notes_uc: notes_uc.clone(),
        reminders_uc: reminders_uc.clone(),
        strikes_uc: strikes_uc.clone(),
        moderation_copilot_uc: moderation_copilot_uc.clone(),
        assess_target_risk_uc: assess_target_risk_uc.clone(),
        automod_reviews_uc: automod_reviews_uc.clone(),
        automod_adaptive_slowmode_repo: automod_adaptive_slowmode_repo.clone(),
        sursis_uc: sursis_uc.clone(),
        cancel_action_uc: Arc::new(
            sentinel_core::application::moderation::cancel_action_service::CancelModerationActionService::new(
                moderation_uc.clone(),
                reminders_uc.clone(),
                discord_api.clone(),
            ),
        ),
        evidence_repo: Arc::new(PgEvidenceRepository::new(pg_pool.clone())),
        review_repo: Arc::new(PgReviewRepository::new(pg_pool.clone())),
        pending_action_repo: Arc::new(PgPendingActionRepository::new(pg_pool.clone())),
        modstats_repo: modstats_repo.clone(),
        broadcaster: broadcaster.clone(),
        discord_api: discord_api.clone(),
        bot_config_repo: bot_config_repo.clone(),
    };

    let system = crate::bootstrap::state::SystemState {
        tickets_uc: tickets_uc.clone(),
        system_logs_uc: system_logs_uc.clone(),
        server_events_uc: server_events_uc.clone(),
        reset_guild_uc: reset_guild_uc.clone(),
        bot_persistence_uc: bot_persistence_uc.clone(),
        alert_rules_uc: alert_rules_uc.clone(),
        oauth_uc: oauth_uc.clone(),
        ip_bans_uc: ip_bans_uc.clone(),
        quarantine_uc: quarantine_uc.clone(),
        lockdown_uc: lockdown_uc.clone(),
        slowmode_uc: slowmode_uc.clone(),
        security_logs_uc: security_logs_uc.clone(),
        security_audit_uc: security_audit_uc.clone(),
        host_probe_uc: host_probe_uc.clone(),
        tls_cert_uc: tls_cert_uc.clone(),
        geoip_uc: geoip_uc.clone(),
        export_uc: Arc::new(ExportService::new(Arc::new(
            crate::adapters::outbound::postgres::system::export_repository::PgExportRepository::new(
                pg_pool.clone(),
            ),
        ))),
        export_jobs_uc: Arc::new(
            sentinel_core::application::system::manage_export_jobs_service::ManageExportJobsService::new(
                Arc::new(
                    crate::adapters::outbound::postgres::system::export_job_repository::PgExportJobRepository::new(
                        pg_pool.clone(),
                    ),
                ),
            ),
        ),
        docker_host: docker_host.clone(),
        system_probe: Arc::new(
            crate::adapters::outbound::system::pg_probe::PgSystemProbe::new(pg_pool.clone()),
        ),
        guild_repo: guild_repo.clone(),
        log_repo: log_repo.clone(),
        container_monitor: Some(crate::bootstrap::container_monitor::spawn(pg_pool.clone())),
        rate_limiter: Some(Arc::new(
            crate::adapters::outbound::system::rate_limiter::RateLimiter::from_env(),
        )),
        broadcaster: broadcaster.clone(),
        discord_api: discord_api.clone(),
        bot_config_repo: bot_config_repo.clone(),
        redis_client: redis_client.clone(),
        discord_oauth_client_id: config.discord_oauth_client_id.clone(),
        discord_oauth_client_secret: config.discord_oauth_client_secret.clone(),
        discord_oauth_redirect_uri: config.discord_oauth_redirect_uri.clone(),
        web_front_url: config.web_front_url.clone(),
        superadmin_user_ids: Arc::new(config.superadmin_user_ids.clone()),
        api_key: config.api_key.clone(),
    };

    let community = crate::bootstrap::state::CommunityState {
        events_uc: events_uc.clone(),
        lfg_uc: lfg_uc.clone(),
        polls_uc: polls_uc.clone(),
        spotlight_uc: spotlight_uc.clone(),
        news_uc: news_uc.clone(),
        ideas_uc: ideas_uc.clone(),
        confessions_uc: confessions_uc.clone(),
        announcements_uc: announcements_uc.clone(),
        embeds_uc: embeds_uc.clone(),
        presence_uc: presence_uc.clone(),
        members_uc: members_uc.clone(),
        levels_uc: levels_uc.clone(),
        monthly_ranking_uc: monthly_ranking_uc.clone(),
        role_panels_uc: role_panels_uc.clone(),
        voice_channels_uc: voice_channels_uc.clone(),
        welcome_config_uc: welcome_config_uc.clone(),
        eligibility_uc: eligibility_uc.clone(),
        age_check_uc: age_check_uc.clone(),
        manage_sponsorships_uc: Arc::new(
            sentinel_core::application::community::manage_sponsorships_service::ManageSponsorshipsService::new(
                Arc::new(PgSponsorshipRepository::new(pg_pool.clone())),
                Arc::new(PgTempRoleRepository::new(pg_pool.clone())),
            ),
        ),
        daily_activity_repo: daily_activity_repo.clone(),
        discord_role_repo: discord_role_repo.clone(),
        age_ban_repo: age_ban_repo.clone(),
        sponsorship_repo: Arc::new(PgSponsorshipRepository::new(pg_pool.clone())),
        temp_role_repo: Arc::new(PgTempRoleRepository::new(pg_pool.clone())),
        broadcaster: broadcaster.clone(),
        discord_api: discord_api.clone(),
        bot_config_repo: bot_config_repo.clone(),
        redis_client: redis_client.clone(),
    };

    let audit = crate::bootstrap::state::AuditState {
        audit_logs_uc: audit_logs_uc.clone(),
        watched_users_uc: watched_users_uc.clone(),
        stats_uc: stats_uc.clone(),
        detect_anomaly_uc: detect_anomaly_uc.clone(),
        weekly_report_uc: weekly_report_uc.clone(),
        snapshots_uc: snapshots_uc.clone(),
        discord_action_messages_uc: discord_action_messages_uc.clone(),
        security_uc: security_uc.clone(),
        analytics_repo: analytics_repo.clone(),
        user_activity_repo: user_activity_repo.clone(),
        broadcaster: broadcaster.clone(),
        bot_config_repo: bot_config_repo.clone(),
        redis_client: redis_client.clone(),
        daily_activity_repo: daily_activity_repo.clone(),
        discord_api: discord_api.clone(),
    };

    let guild_backup = crate::bootstrap::state::GuildBackupState {
        guild_snapshots_uc: guild_snapshots_uc.clone(),
        pending_role_grants_uc: pending_role_grants_uc.clone(),
        bot_config_repo: bot_config_repo.clone(),
        broadcaster: broadcaster.clone(),
    };

    AppState {
        ai,
        moderation: moderation.clone(),
        audit: audit.clone(),
        community: community.clone(),
        system: system.clone(),
        guild_backup: guild_backup.clone(),
        log_repo,
        bot_config_repo,
        broadcaster,
        job_client,
        discord_api,
        api_key: config.api_key.clone(),
        guild_id: config.guild_id.clone(),
        nexus_games: Arc::new(
            crate::adapters::outbound::nexus_games::NexusGamesClient::new(
                config.nexus_api_url.clone(),
                config.nexus_api_key.clone(),
            ),
        ),
        metrics_token: config.metrics_token.clone(),
        discord_bot_token: config.discord_bot_token.clone(),
        pg_pool: pg_pool.clone(),
        redis_client: redis_client.clone(),
        cache: Some(cache.clone()),
        superadmin_user_ids: Arc::new(config.superadmin_user_ids.clone()),
    }
}
