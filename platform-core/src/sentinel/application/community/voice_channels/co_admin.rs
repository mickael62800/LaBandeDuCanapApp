use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelCoAdmin;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;

use super::ManageVoiceChannelsService;

impl ManageVoiceChannelsService {
    pub(super) async fn add_co_admin_impl(
        &self,
        cmd: ManageCoAdminCommand,
    ) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        let co_admin = VoiceChannelCoAdmin {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            user_id: cmd.user_id,
            user_name: cmd.user_name,
            granted_at: Utc::now(),
        };

        self.repo.add_co_admin(&co_admin).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id)
            .await;
        Ok(())
    }

    pub(super) async fn remove_co_admin_impl(
        &self,
        channel_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.remove_co_admin(channel.id, user_id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }
}
