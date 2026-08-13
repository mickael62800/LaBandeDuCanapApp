//! Use case Welcome / Onboarding (cf. roadmap Phase 3).
//!
//! Le handler HTTP / gRPC appelle ce port inbound — jamais directement
//! le `WelcomeConfigRepository`. Permet d'isoler les regles applicatives
//! (validation, merge partiel, etc.) du transport.

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigData;

/// Patch partiel : chaque champ `Some` ecrase, `None` conserve l existant.
/// Equivalent du `SaveWelcomeConfigDto` cote HTTP, mais decoupe du DTO
/// pour rester reutilisable depuis gRPC ou un autre adapter.
#[derive(Debug, Clone, Default)]
pub struct WelcomeConfigPatch {
    pub welcome_enabled: Option<bool>,
    pub welcome_channel_id: Option<String>,
    pub welcome_message: Option<String>,
    pub welcome_embed_color: Option<String>,
    pub welcome_dm_enabled: Option<bool>,
    pub welcome_dm_message: Option<String>,
    pub welcome_title: Option<String>,
    pub welcome_image_url: Option<String>,
    pub welcome_footer_text: Option<String>,
    pub leave_enabled: Option<bool>,
    pub leave_channel_id: Option<String>,
    pub leave_message: Option<String>,
    pub leave_title: Option<String>,
    pub leave_image_url: Option<String>,
    pub leave_footer_text: Option<String>,
    pub rules_enabled: Option<bool>,
    pub rules_channel_id: Option<String>,
    pub rules_message: Option<String>,
    pub rules_role_id: Option<String>,
    pub rules_button_label: Option<String>,
    pub age_check_enabled: Option<bool>,
    pub age_minimum: Option<i32>,
    pub unverified_role_id: Option<String>,
    pub age_modal_question: Option<String>,
    pub age_ban_message: Option<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub age_ban_days_per_year: Option<i32>,
    pub age_ban_log_channel_id: Option<String>,
    pub leave_embed_color: Option<String>,
    pub rules_embed_color: Option<String>,
    pub counter_enabled: Option<bool>,
    pub counter_channel_id: Option<String>,
    pub counter_format: Option<String>,
    pub voice_counter_enabled: Option<bool>,
    pub voice_counter_channel_id: Option<String>,
    pub voice_counter_format: Option<String>,
    pub anniversary_enabled: Option<bool>,
    pub anniversary_channel_id: Option<String>,
    pub anniversary_message: Option<String>,
    pub anniversary_title: Option<String>,
    pub anniversary_image_url: Option<String>,
    pub anniversary_footer_text: Option<String>,
    pub rejoin_message: Option<String>,
    pub rejoin_title: Option<String>,
    pub rejoin_image_url: Option<String>,
    pub rejoin_footer_text: Option<String>,
}

#[async_trait]
pub trait ManageWelcomeConfigUseCase: Send + Sync {
    async fn get(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError>;
    async fn save_patch(
        &self,
        guild_id: &str,
        patch: WelcomeConfigPatch,
    ) -> Result<WelcomeConfigData, DomainError>;
}
