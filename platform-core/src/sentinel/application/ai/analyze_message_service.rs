use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;
use tracing::info;
use tracing::warn;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::message_analysis::MessageAnalysis;
use crate::sentinel::domain::entities::moderation::infraction::Infraction;
use crate::sentinel::domain::enums::moderation::action::Action;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::domain::services::ai::inference_limiter::InferenceRateLimiter;
use crate::sentinel::domain::services::moderation::channel_tension::ChannelTensionBuffer;
use crate::sentinel::domain::services::moderation::channel_tension::TensionAction;
use crate::sentinel::domain::services::moderation::channel_tension::TensionEntry;
use crate::sentinel::domain::services::moderation::scoring_service::ScoringConfig;
use crate::sentinel::domain::services::moderation::scoring_service::ScoringService;
use crate::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand;
use crate::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageUseCase;
use crate::sentinel::ports::outbound::ai::inference_service::InferenceService;
use crate::sentinel::ports::outbound::ai::text_tokenizer::TextTokenizer;
use crate::sentinel::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::sentinel::ports::outbound::moderation::rule_repository::RuleRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;

pub struct AnalyzeMessageService {
    rule_repo: Arc<dyn RuleRepository>,
    infraction_repo: Arc<dyn InfractionRepository>,
    cache: Arc<dyn CachePort>,
    /// Repo pour lire la config automod-bot : cles IA (text_enabled,
    /// text_threshold, context_dampening, context_format) + cles tension
    /// de salon (activation + seuils). Anciennement lu depuis la table
    /// dediee `ia_config` ; fusion dans automod-bot via migration 146.
    bot_config_repo: Arc<dyn BotConfigRepository>,
    inference_limiter: Arc<InferenceRateLimiter>,
    inference: Option<Arc<dyn InferenceService>>,
    tokenizer: Option<Arc<dyn TextTokenizer>>,
    deepseek_service: Option<
        Arc<dyn crate::sentinel::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationService>,
    >,
    /// Buffer in-memory pour la "tension de salon" (option : si None, la
    /// feature est desactivee quel que soit le contenu de la config).
    tension_buffer: Option<Arc<ChannelTensionBuffer>>,
}

impl AnalyzeMessageService {
    pub fn new(
        rule_repo: Arc<dyn RuleRepository>,
        infraction_repo: Arc<dyn InfractionRepository>,
        cache: Arc<dyn CachePort>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
        inference_limiter: Arc<InferenceRateLimiter>,
    ) -> Self {
        Self {
            rule_repo,
            infraction_repo,
            cache,
            bot_config_repo,
            inference_limiter,
            inference: None,
            tokenizer: None,
            deepseek_service: None,
            tension_buffer: None,
        }
    }

    /// Ajoute le service DeepSeek Moderation au pipeline.
    pub fn with_deepseek(
        mut self,
        deepseek: Arc<
            dyn crate::sentinel::ports::outbound::ai::deepseek_moderation_service::DeepSeekModerationService,
        >,
    ) -> Self {
        self.deepseek_service = Some(deepseek);
        self
    }

    /// Ajoute l'inference text IA au service d'analyse.
    pub fn with_text_inference(
        mut self,
        inference: Arc<dyn InferenceService>,
        tokenizer: Arc<dyn TextTokenizer>,
    ) -> Self {
        self.inference = Some(inference);
        self.tokenizer = Some(tokenizer);
        self
    }

    /// Ajoute la feature "tension de salon" (buffer glissant + seuils
    /// lus depuis `bot_guild_config` pour `automod-bot`).
    pub fn with_channel_tension(mut self, buffer: Arc<ChannelTensionBuffer>) -> Self {
        self.tension_buffer = Some(buffer);
        self
    }
}

mod config;
mod heuristics;
mod pipeline;
mod scoring;


#[cfg(test)]
use crate::sentinel::domain::services::moderation::scoring_service::resolve_thresholds;
#[cfg(test)]
use config::DEFAULT_TEXT_THRESHOLD;
pub(crate) use config::{parse_ia_config_from_bot_config, parse_scoring_config};
use config::{parse_tension_config, tension_is_stronger};
use scoring::build_contextual_content;
pub(crate) use scoring::cap_ia_induced_ban;
pub use scoring::{score_classifications, score_deepseek_analysis};

#[async_trait]
impl AnalyzeMessageUseCase for AnalyzeMessageService {
    async fn evaluate_flood(
        &self,
        guild_id: &str,
        flood_count: i32,
    ) -> Result<crate::sentinel::ports::inbound::ai::analyze_message::FloodDecision, DomainError>
    {
        self.evaluate_flood_impl(guild_id, flood_count).await
    }

    async fn evaluate_attachments(
        &self,
        guild_id: &str,
        filenames: Vec<String>,
    ) -> Result<crate::sentinel::ports::inbound::ai::analyze_message::AttachmentDecision, DomainError>
    {
        self.evaluate_attachments_impl(guild_id, filenames).await
    }

    async fn evaluate_caps(
        &self,
        guild_id: &str,
    ) -> Result<crate::sentinel::ports::inbound::ai::analyze_message::CapsDecision, DomainError>
    {
        self.evaluate_caps_impl(guild_id).await
    }

    async fn analyze(&self, cmd: AnalyzeMessageCommand) -> Result<MessageAnalysis, DomainError> {
        self.analyze_impl(cmd).await
    }
}

#[cfg(test)]
#[path = "tests/analyze_message_service.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/analyze_message_pipeline.rs"]
mod tests_pipeline;

#[cfg(test)]
#[path = "tests/analyze_message_heuristics.rs"]
mod tests_heuristics;
