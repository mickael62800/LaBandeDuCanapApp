//! Service application Welcome — orchestre le merge patch <-> repo.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase;
use crate::sentinel::ports::inbound::community::manage_welcome_config::WelcomeConfigPatch;
use crate::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigData;
use crate::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;
pub struct ManageWelcomeConfigService {
    repo: Arc<dyn WelcomeConfigRepository>,
}

impl ManageWelcomeConfigService {
    pub fn new(repo: Arc<dyn WelcomeConfigRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageWelcomeConfigUseCase for ManageWelcomeConfigService {
    async fn get(&self, guild_id: &str) -> Result<WelcomeConfigData, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        self.repo.get_config(guild_id).await
    }

    async fn save_patch(
        &self,
        guild_id: &str,
        patch: WelcomeConfigPatch,
    ) -> Result<WelcomeConfigData, DomainError> {
        crate::sentinel::application::validation::validate_guild_id(guild_id)?;
        // Merge : on lit la config actuelle puis on applique les champs
        // presents (le `None` conserve l existant).
        let mut current = self.repo.get_config(guild_id).await?;
        if let Some(v) = patch.welcome_enabled {
            current.welcome_enabled = v;
        }
        if let Some(v) = patch.welcome_channel_id {
            current.welcome_channel_id = Some(v);
        }
        if let Some(v) = patch.welcome_message {
            current.welcome_message = v;
        }
        if let Some(v) = patch.welcome_embed_color {
            current.welcome_embed_color = v;
        }
        if let Some(v) = patch.welcome_dm_enabled {
            current.welcome_dm_enabled = v;
        }
        if let Some(v) = patch.welcome_dm_message {
            current.welcome_dm_message = v;
        }
        if let Some(v) = patch.welcome_title {
            current.welcome_title = v;
        }
        if let Some(v) = patch.welcome_image_url {
            current.welcome_image_url = v;
        }
        if let Some(v) = patch.welcome_footer_text {
            current.welcome_footer_text = v;
        }
        if let Some(v) = patch.leave_enabled {
            current.leave_enabled = v;
        }
        if let Some(v) = patch.leave_channel_id {
            current.leave_channel_id = Some(v);
        }
        if let Some(v) = patch.leave_message {
            current.leave_message = v;
        }
        if let Some(v) = patch.leave_title {
            current.leave_title = v;
        }
        if let Some(v) = patch.leave_image_url {
            current.leave_image_url = v;
        }
        if let Some(v) = patch.leave_footer_text {
            current.leave_footer_text = v;
        }
        if let Some(v) = patch.rules_enabled {
            current.rules_enabled = v;
        }
        if let Some(v) = patch.rules_channel_id {
            current.rules_channel_id = Some(v);
        }
        if let Some(v) = patch.rules_message {
            current.rules_message = v;
        }
        if let Some(v) = patch.rules_role_id {
            current.rules_role_id = Some(v);
        }
        if let Some(v) = patch.rules_button_label {
            current.rules_button_label = v;
        }
        if let Some(v) = patch.age_check_enabled {
            current.age_check_enabled = v;
        }
        if let Some(v) = patch.age_minimum {
            current.age_minimum = v;
        }
        if let Some(v) = patch.unverified_role_id {
            current.unverified_role_id = Some(v);
        }
        if let Some(v) = patch.age_modal_question {
            current.age_modal_question = v;
        }
        if let Some(v) = patch.age_ban_message {
            current.age_ban_message = v;
        }
        if let Some(v) = patch.age_min {
            current.age_min = v;
        }
        if let Some(v) = patch.age_max {
            current.age_max = v;
        }
        if let Some(v) = patch.age_ban_days_per_year {
            current.age_ban_days_per_year = v;
        }
        if let Some(v) = patch.age_ban_log_channel_id {
            current.age_ban_log_channel_id = Some(v);
        }
        if let Some(v) = patch.leave_embed_color {
            current.leave_embed_color = v;
        }
        if let Some(v) = patch.rules_embed_color {
            current.rules_embed_color = v;
        }
        if let Some(v) = patch.counter_enabled {
            current.counter_enabled = v;
        }
        if let Some(v) = patch.counter_channel_id {
            current.counter_channel_id = Some(v);
        }
        if let Some(v) = patch.counter_format {
            current.counter_format = v;
        }
        if let Some(v) = patch.voice_counter_enabled {
            current.voice_counter_enabled = v;
        }
        if let Some(v) = patch.voice_counter_channel_id {
            current.voice_counter_channel_id = Some(v);
        }
        if let Some(v) = patch.voice_counter_format {
            current.voice_counter_format = v;
        }
        if let Some(v) = patch.anniversary_enabled {
            current.anniversary_enabled = v;
        }
        if let Some(v) = patch.anniversary_channel_id {
            current.anniversary_channel_id = Some(v);
        }
        if let Some(v) = patch.anniversary_message {
            current.anniversary_message = v;
        }
        if let Some(v) = patch.anniversary_title {
            current.anniversary_title = v;
        }
        if let Some(v) = patch.anniversary_image_url {
            current.anniversary_image_url = v;
        }
        if let Some(v) = patch.anniversary_footer_text {
            current.anniversary_footer_text = v;
        }
        if let Some(v) = patch.rejoin_message {
            current.rejoin_message = v;
        }
        if let Some(v) = patch.rejoin_title {
            current.rejoin_title = v;
        }
        if let Some(v) = patch.rejoin_image_url {
            current.rejoin_image_url = v;
        }
        if let Some(v) = patch.rejoin_footer_text {
            current.rejoin_footer_text = v;
        }

        self.repo.save_config(guild_id, &current).await
    }
}
