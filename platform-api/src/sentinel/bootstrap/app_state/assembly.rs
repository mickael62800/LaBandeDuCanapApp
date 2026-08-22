{
// ── Sous-etats par domaine ──
//
// Construits AVANT le `AppState` plat pour que ce dernier puisse cloner
// depuis eux : une seule source par port, pas deux instanciations qui
// divergeraient silencieusement. Cf. `bootstrap::state` pour le pourquoi
// de la coexistence des deux formes pendant la migration.
let ai = crate::sentinel::bootstrap::state::AiState {
    analyze_uc: analyze_uc.clone(),
    analyze_image_uc: analyze_image_uc.clone(),
    dataset_uc: dataset_uc.clone(),
    ai_jobs_uc: ai_jobs_uc.clone(),
    inference: inference.clone(),
    broadcaster: broadcaster.clone(),
};

let moderation = crate::sentinel::bootstrap::state::ModerationState {
    rules_uc: rules_uc.clone(),
    infractions_uc: infractions_uc.clone(),
    moderation_uc: moderation_uc.clone(),
    modstats_uc: modstats_uc.clone(),
    assess_target_risk_uc: assess_target_risk_uc.clone(),
    automod_reviews_uc: automod_reviews_uc.clone(),
    automod_adaptive_slowmode_repo: automod_adaptive_slowmode_repo.clone(),
    sursis_uc: sursis_uc.clone(),
    strikes_uc: strikes_uc.clone(),
    cancel_action_uc: Arc::new(
        platform_core::sentinel::application::moderation::cancel_action_service::CancelModerationActionService::new(
            moderation_uc.clone(),
            discord_api.clone(),
        ),
    ),
    evidence_repo: Arc::new(PgEvidenceRepository::new(pg_pool.clone())),
    review_repo: Arc::new(PgReviewRepository::new(pg_pool.clone())),
    pending_action_repo: Arc::new(PgPendingActionRepository::new(pg_pool.clone())),
    modstats_repo: modstats_repo.clone(),
    manage_reminders_uc: Arc::new(platform_core::sentinel::application::moderation::manage_reminders_service::ManageRemindersService::new(
        Arc::new(crate::sentinel::adapters::outbound::postgres::moderation::reminder_repository::PgReminderRepository::new(pg_pool.clone()))
    )),
    notes_uc: Arc::new(platform_core::sentinel::application::moderation::manage_notes_service::ManageNotesService::new(
        Arc::new(crate::sentinel::adapters::outbound::postgres::moderation::notes_repository::PgNotesRepository::new(pg_pool.clone()))
    )),
    broadcaster: broadcaster.clone(),
    discord_api: discord_api.clone(),
    bot_config_repo: bot_config_repo.clone(),
};

// Exploitation de la machine hote : transverse aux trois plateformes,
// donc distinct du metier Discord porte par SystemState.
let ops = crate::sentinel::bootstrap::state::OpsState {
    system_probe: Arc::new(
        crate::sentinel::adapters::outbound::system::pg_probe::PgSystemProbe::new(pg_pool.clone()),
    ),
    service_registry: service_registry.clone(),
    rate_limiter: Some(Arc::new(
        crate::sentinel::adapters::outbound::system::rate_limiter::RateLimiter::from_env(),
    )),
    broadcaster: broadcaster.clone(),
    redis_client: redis_client.clone(),
};

let system = crate::sentinel::bootstrap::state::SystemState {
    tickets_uc: tickets_uc.clone(),
    reset_guild_uc: reset_guild_uc.clone(),
    bot_persistence_uc: bot_persistence_uc.clone(),
    quarantine_uc: quarantine_uc.clone(),
    lockdown_uc: lockdown_uc.clone(),
    slowmode_uc: slowmode_uc.clone(),
    export_uc: Arc::new(ExportService::new(Arc::new(
        crate::sentinel::adapters::outbound::postgres::system::export_repository::PgExportRepository::new(
            pg_pool.clone(),
        ),
    ))),
    export_jobs_uc: Arc::new(
        platform_core::sentinel::application::system::manage_export_jobs_service::ManageExportJobsService::new(
            Arc::new(
                crate::sentinel::adapters::outbound::postgres::system::export_job_repository::PgExportJobRepository::new(
                    pg_pool.clone(),
                ),
            ),
        ),
    ),
    guild_repo: guild_repo.clone(),
    broadcaster: broadcaster.clone(),
    discord_api: discord_api.clone(),
    bot_config_repo: bot_config_repo.clone(),
    redis_client: redis_client.clone(),
    api_key: config.api_key.clone(),
};

let member_repo = Arc::new(
    crate::sentinel::adapters::outbound::postgres::community::member_repository::PgMemberRepository::new(
        pg_pool.clone(),
    ),
);
let members_uc: Arc<
    dyn platform_core::sentinel::ports::inbound::community::manage_members::ManageMembersUseCase,
> = Arc::new(
    platform_core::sentinel::application::community::manage_members_service::ManageMembersService::new(
        member_repo,
        infractions_uc.clone(),
        moderation_uc.clone(),
        stats_uc.clone(),
    ),
);

let role_panels_uc: Arc<
    dyn platform_core::sentinel::ports::inbound::community::manage_role_panels::ManageRolePanelsUseCase,
> = Arc::new(
    platform_core::sentinel::application::community::manage_role_panels_service::ManageRolePanelsService::new(
        Arc::new(crate::sentinel::adapters::outbound::postgres::community::role_panel_repository::PgRolePanelRepository::new(pg_pool.clone())),
    ),
);

let voice_channels_uc: Arc<
    dyn platform_core::sentinel::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase,
> = Arc::new(
    platform_core::sentinel::application::community::voice_channels::ManageVoiceChannelsService::new(
        Arc::new(crate::sentinel::adapters::outbound::postgres::community::voice_channel_repository::PgVoiceChannelRepository::new(pg_pool.clone())),
        cache.clone(),
        bot_config_repo.clone(),
    ),
);

let community = crate::sentinel::bootstrap::state::CommunityState {
    events_uc: events_uc.clone(),
    lfg_uc: lfg_uc.clone(),
    members_uc: members_uc.clone(),
    role_panels_uc: role_panels_uc.clone(),
    // Delai d'acceptation du reglement : le service lit le reglage de la guilde
    // (`welcome-bot`) et calcule l'echeance, le repo ne fait que persister.
    rules_deadline_uc: Arc::new(
        platform_core::sentinel::application::community::manage_rules_deadline_service::ManageRulesDeadlineService::new(
            Arc::new(crate::sentinel::adapters::outbound::postgres::community::rules_deadline_repository::PgRulesDeadlineRepository::new(pg_pool.clone())),
            bot_config_repo.clone(),
        ),
    ),
    voice_channels_uc: voice_channels_uc.clone(),
    polls_uc: polls_uc.clone(),
    spotlight_uc: spotlight_uc.clone(),
    news_uc: news_uc.clone(),
    ideas_uc: ideas_uc.clone(),
    confessions_uc: confessions_uc.clone(),
    announcements_uc: announcements_uc.clone(),
    embeds_uc: embeds_uc.clone(),
    presence_uc: presence_uc.clone(),
    levels_uc: manage_levels_usecase.clone(),
    monthly_ranking_uc: monthly_ranking_uc.clone(),
    welcome_config_uc: welcome_config_uc.clone(),
    eligibility_uc: eligibility_uc.clone(),
    age_check_uc: age_check_uc.clone(),
    manage_sponsorships_uc: Arc::new(
        platform_core::sentinel::application::community::manage_sponsorships_service::ManageSponsorshipsService::new(
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

let audit = crate::sentinel::bootstrap::state::AuditState {
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

let guild_backup = crate::sentinel::bootstrap::state::GuildBackupState {
    guild_snapshots_uc: guild_snapshots_uc.clone(),
    pending_role_grants_uc: pending_role_grants_uc.clone(),
    bot_config_repo: bot_config_repo.clone(),
    broadcaster: broadcaster.clone(),
    api_key: config.api_key.clone(),
};

let shared = crate::sentinel::bootstrap::state::SharedState {
    log_repo,
    broadcaster,
    redis_client: redis_client.clone(),
    cache: Some(cache),
    nexus_games: Arc::new(
        crate::sentinel::adapters::outbound::nexus_games::NexusGamesClient::new(
            config.nexus_api_url.clone(),
            config.nexus_api_key.clone(),
        ),
    ),
    api_key: config.api_key.clone(),
    guild_id: config.guild_id.clone(),
    metrics_token: config.metrics_token.clone(),
    auth: Arc::new(crate::shared::auth_client::AuthClient::new(
        std::env::var("AUTH_API_URL").unwrap_or_else(|_| "http://auth-api:8096".into()),
        std::env::var("AUTH_API_TOKEN").unwrap_or_default(),
    )),
};

let jobs = crate::sentinel::bootstrap::state::InternalJobsState {
    runner: Arc::new(crate::sentinel::jobs::internal_runner::InternalJobRunner::new(
        pg_pool.clone(),
        redis_client.clone(),
    )),
    job_lock_pool: pg_pool.clone(),
};

AppState {
    ai,
    ops: ops.clone(),
    moderation: moderation.clone(),
    audit: audit.clone(),
    community: community.clone(),
    system: system.clone(),
    guild_backup: guild_backup.clone(),
    shared,
    jobs,
}
}
