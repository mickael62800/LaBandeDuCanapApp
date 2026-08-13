use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelConfig;
use crate::sentinel::domain::errors::DomainError;

use super::ManageVoiceChannelsService;

impl ManageVoiceChannelsService {
    // ── Config voice-bot par guild ──

    pub(super) async fn get_voice_config_impl(
        &self,
        guild_id: &str,
    ) -> Result<VoiceChannelConfig, DomainError> {
        let entries = self
            .bot_config_repo
            .get_config(guild_id, "voice-bot")
            .await?;
        let pairs: Vec<(String, String)> = entries
            .into_iter()
            .map(|e| (e.config_key, e.config_value))
            .collect();
        Ok(VoiceChannelConfig::from_kv_pairs(&pairs))
    }
}
