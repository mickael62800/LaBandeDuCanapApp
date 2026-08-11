{
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

(
    discord_api,
    inference,
    analyze_uc,
    analyze_image_uc,
    dataset_uc,
    ai_jobs_uc,
)
}
