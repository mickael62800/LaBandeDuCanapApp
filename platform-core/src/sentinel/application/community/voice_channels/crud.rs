use chrono::Utc;
use uuid::Uuid;

use super::ManageVoiceChannelsService;
use super::CHANNELS_LIST_TTL;
use super::CHANNEL_DETAIL_TTL;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannel;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelDetail;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_voice_channels::CreateVoiceChannelCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use crate::sentinel::ports::outbound::system::cache_helpers::cached_json;
impl ManageVoiceChannelsService {
    pub(super) async fn list_all_channels_impl(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        self.repo.find_all().await
    }

    pub(super) async fn list_channels_impl(
        &self,
        guild_id: &str,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        let cache_key = format!("voice_channels:{guild_id}");
        cached_json(&self.cache, &cache_key, CHANNELS_LIST_TTL, || async {
            self.repo.find_all_by_guild(guild_id).await
        })
        .await
    }

    pub(super) async fn list_history_channels_impl(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        // Historique : pas de cache — donnees moins sollicitees et
        // fraicheur preferable.
        self.repo.find_closed_by_guild(guild_id, limit).await
    }

    pub(super) async fn get_channel_detail_impl(
        &self,
        channel_id: &str,
    ) -> Result<VoiceChannelDetail, DomainError> {
        let cache_key = format!("voice_channel:{channel_id}");
        cached_json(&self.cache, &cache_key, CHANNEL_DETAIL_TTL, || async {
            let channel = self.resolve_channel(channel_id).await?;
            let co_admins = self.repo.find_co_admins(channel.id).await?;
            let bans = self
                .repo
                .find_bans_for_owner(channel.guild_id.as_str(), &channel.owner_id)
                .await?;
            let invite_links = self.repo.find_invite_links(channel.id).await?;
            Ok(VoiceChannelDetail {
                channel,
                co_admins,
                bans,
                invite_links,
            })
        })
        .await
    }

    pub(super) async fn create_channel_impl(
        &self,
        cmd: CreateVoiceChannelCommand,
    ) -> Result<VoiceChannel, DomainError> {
        let channel = VoiceChannel {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            owner_id: cmd.owner_id,
            owner_name: cmd.owner_name,
            channel_id: cmd.channel_id,
            text_channel_id: cmd.text_channel_id,
            members_channel_id: cmd.members_channel_id,
            queue_channel_id: cmd.queue_channel_id,
            category_id: cmd.category_id,
            channel_name: cmd.channel_name,
            kind: crate::sentinel::domain::enums::community::voice_channel_kind::VoiceChannelKind::from_str_lossy(&cmd.kind),
            visibility: cmd.visibility,
            queue_enabled: cmd.queue_enabled,
            locked: false,
            stage_enabled: cmd.stage_enabled,
            member_limit: None,
            status: None,
            channel_status: "open".to_string(),
            closed_at: None,
            created_at: Utc::now(),
        };

        self.repo.save(&channel).await?;
        if let Err(e) = self
            .cache
            .invalidate(&format!("voice_channels:{}", channel.guild_id))
            .await
        {
            tracing::warn!(error = %e, guild_id = %channel.guild_id, "Echec invalidation cache voice_channels apres creation");
        }

        Ok(channel)
    }

    pub(super) async fn close_channel_impl(&self, channel_id: &str) -> Result<(), DomainError> {
        self.repo.close_by_channel_id(channel_id).await?;
        // Invalider le cache — on essaie de résoudre le channel pour le guild_id
        if let Ok(channel) = self.resolve_channel(channel_id).await {
            self.invalidate_cache(&channel.guild_id, channel_id).await;
        }
        Ok(())
    }

    pub(super) async fn delete_channel_impl(&self, channel_id: &str) -> Result<(), DomainError> {
        // Soft-delete : close au lieu de supprimer
        self.close_channel_impl(channel_id).await
    }

    pub(super) async fn find_guild_id_impl(
        &self,
        channel_id: &str,
    ) -> Result<Option<String>, DomainError> {
        Ok(self
            .repo
            .find_by_channel_id(channel_id)
            .await?
            .map(|c| c.guild_id.as_str().to_string()))
    }

    pub(super) async fn purge_channel_impl(&self, channel_id: &str) -> Result<bool, DomainError> {
        // On resout la guild AVANT la suppression pour pouvoir invalider le cache.
        let guild_id = self
            .repo
            .find_by_channel_id(channel_id)
            .await?
            .map(|c| c.guild_id.as_str().to_string());

        let deleted = self
            .repo
            .hard_delete_closed_by_channel_id(channel_id)
            .await?;

        if deleted > 0 {
            if let Some(guild_id) = guild_id {
                self.invalidate_cache(&guild_id, channel_id).await;
            }
        }
        Ok(deleted > 0)
    }

    pub(super) async fn purge_history_impl(&self, guild_id: &str) -> Result<u64, DomainError> {
        let count = self.repo.hard_delete_closed_by_guild(guild_id).await?;
        if let Err(e) = self
            .cache
            .invalidate(&format!("voice_channels:{guild_id}"))
            .await
        {
            tracing::warn!(error = %e, guild_id, "Echec invalidation cache voice_channels apres purge historique");
        }
        Ok(count)
    }

    pub(super) async fn update_channel_impl(
        &self,
        cmd: UpdateVoiceChannelCommand,
    ) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;

        if let Some(vis) = &cmd.visibility {
            self.repo.update_visibility(channel.id, vis).await?;
        }
        if let Some(locked) = cmd.locked {
            self.repo.update_locked(channel.id, locked).await?;
        }
        if let Some(queue_enabled) = cmd.queue_enabled {
            self.repo
                .update_queue_enabled(channel.id, queue_enabled)
                .await?;
        }
        if let Some(name) = &cmd.name {
            self.repo.update_name(channel.id, name).await?;
        }
        if let Some(status) = &cmd.status {
            self.repo.update_status(channel.id, Some(status)).await?;
        }
        if let Some(limit) = cmd.member_limit {
            self.repo.update_member_limit(channel.id, limit).await?;
        }
        if let Some(queue_ch) = &cmd.queue_channel_id {
            self.repo
                .update_queue_channel(channel.id, queue_ch.as_deref())
                .await?;
        }
        if let Some(stage) = cmd.stage_enabled {
            self.repo.update_stage(channel.id, stage).await?;
        }

        self.invalidate_cache(&channel.guild_id, &cmd.channel_id)
            .await;
        Ok(())
    }

    pub(super) async fn transfer_ownership_impl(
        &self,
        cmd: TransferOwnershipCommand,
    ) -> Result<(), DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;
        self.repo
            .update_owner(channel.id, &cmd.new_owner_id, &cmd.new_owner_name)
            .await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id)
            .await;
        Ok(())
    }
}
