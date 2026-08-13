use chrono::Utc;
use uuid::Uuid;

use super::ManageVoiceChannelsService;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;

impl ManageVoiceChannelsService {
    // ── Invite Links ──

    pub(super) async fn create_invite_link_impl(
        &self,
        cmd: CreateInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        let channel = self.resolve_channel(&cmd.channel_id).await?;
        let duration_secs = cmd.duration_secs.unwrap_or(1800);
        let expires_at = Utc::now() + chrono::Duration::seconds(duration_secs);
        let code = self.generate_unique_code().await?;

        let link = VoiceChannelInviteLink {
            id: Uuid::new_v4(),
            voice_channel_id: channel.id,
            guild_id: channel.guild_id.clone(),
            channel_id: channel.channel_id.clone(),
            created_by: cmd.created_by,
            created_by_name: cmd.created_by_name,
            code,
            max_uses: cmd.max_uses,
            current_uses: 0,
            expires_at,
            revoked: false,
            created_at: Utc::now(),
        };

        self.repo.save_invite_link(&link).await?;
        self.invalidate_cache(&channel.guild_id, &cmd.channel_id)
            .await;

        Ok(link)
    }

    pub(super) async fn list_invite_links_impl(
        &self,
        channel_id: &str,
    ) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        self.repo.find_invite_links(channel.id).await
    }

    pub(super) async fn use_invite_link_impl(
        &self,
        cmd: UseInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        let mut link = self
            .repo
            .find_invite_by_code(&cmd.code)
            .await?
            .ok_or_else(|| {
                DomainError::NotFound(format!("Code d'invitation invalide : {}", cmd.code))
            })?;

        if link.revoked {
            return Err(DomainError::ValidationError(
                "Ce lien d'invitation a ete revoque".to_string(),
            ));
        }
        if link.expires_at < Utc::now() {
            return Err(DomainError::ValidationError(
                "Ce lien d'invitation a expire".to_string(),
            ));
        }

        let incremented = self.repo.increment_invite_uses(link.id).await?;
        if !incremented {
            return Err(DomainError::ValidationError(
                "Ce lien d'invitation n'est plus utilisable (limite atteinte ou expire)"
                    .to_string(),
            ));
        }

        // Mettre a jour current_uses pour refléter l'increment
        link.current_uses += 1;

        // Whitelist the user
        let channel = self.resolve_channel(&link.channel_id).await?;
        let entry = VoiceChannelWhitelistEntry {
            id: Uuid::new_v4(),
            guild_id: link.guild_id.clone(),
            owner_id: channel.owner_id.clone(),
            target_id: cmd.user_id.into(),
            target_name: cmd.user_name,
            created_at: Utc::now(),
        };
        self.repo.add_to_whitelist(&entry).await?;
        self.invalidate_cache(&link.guild_id, &link.channel_id)
            .await;

        Ok(link)
    }

    pub(super) async fn revoke_invite_link_impl(
        &self,
        channel_id: &str,
        link_id: &str,
    ) -> Result<(), DomainError> {
        let channel = self.resolve_channel(channel_id).await?;
        let id = Uuid::parse_str(link_id)
            .map_err(|_| DomainError::ValidationError(format!("ID invalide : {link_id}")))?;
        self.repo.revoke_invite_link(id).await?;
        self.invalidate_cache(&channel.guild_id, channel_id).await;
        Ok(())
    }
}
