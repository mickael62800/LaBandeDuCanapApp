use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_voice_channels::CreateThemeCommand;

use super::ManageVoiceChannelsService;

impl ManageVoiceChannelsService {
    // ── Themes ──

    pub(super) async fn list_themes_impl(
        &self,
        guild_id: &str,
    ) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        self.repo.find_themes(guild_id).await
    }

    pub(super) async fn create_theme_impl(
        &self,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        Self::validate_theme(&cmd)?;

        if cmd.is_default {
            self.repo.clear_default_themes(&cmd.guild_id).await?;
        }

        let theme = VoiceChannelTheme {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            name: cmd.name,
            emoji: cmd.emoji,
            channel_name_template: cmd.channel_name_template,
            member_limit: cmd.member_limit,
            visibility: cmd.visibility,
            locked: cmd.locked,
            queue_enabled: cmd.queue_enabled,
            bitrate: cmd.bitrate,
            slowmode_secs: cmd.slowmode_secs,
            stage_enabled: cmd.stage_enabled,
            is_default: cmd.is_default,
            sort_order: cmd.sort_order,
            created_at: Utc::now(),
        };

        self.repo.save_theme(&theme).await?;
        Ok(theme)
    }

    pub(super) async fn update_theme_impl(
        &self,
        theme_id: &str,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        Self::validate_theme(&cmd)?;

        let id = Uuid::parse_str(theme_id)
            .map_err(|_| DomainError::ValidationError(format!("ID invalide : {theme_id}")))?;

        let existing = self
            .repo
            .find_theme(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Theme introuvable : {theme_id}")))?;

        // Verifier que le theme appartient au bon guild
        if existing.guild_id != cmd.guild_id {
            return Err(DomainError::ValidationError(
                "Ce theme n'appartient pas a ce serveur".to_string(),
            ));
        }

        if cmd.is_default {
            self.repo.clear_default_themes(&existing.guild_id).await?;
        }

        let theme = VoiceChannelTheme {
            id,
            guild_id: existing.guild_id,
            name: cmd.name,
            emoji: cmd.emoji,
            channel_name_template: cmd.channel_name_template,
            member_limit: cmd.member_limit,
            visibility: cmd.visibility,
            locked: cmd.locked,
            queue_enabled: cmd.queue_enabled,
            bitrate: cmd.bitrate,
            slowmode_secs: cmd.slowmode_secs,
            stage_enabled: cmd.stage_enabled,
            is_default: cmd.is_default,
            sort_order: cmd.sort_order,
            created_at: existing.created_at,
        };

        self.repo.update_theme(&theme).await?;
        Ok(theme)
    }

    pub(super) async fn delete_theme_impl(
        &self,
        guild_id: &str,
        theme_id: &str,
    ) -> Result<(), DomainError> {
        let id = Uuid::parse_str(theme_id)
            .map_err(|_| DomainError::ValidationError(format!("ID invalide : {theme_id}")))?;

        // Verifier que le theme appartient au bon guild
        let existing = self
            .repo
            .find_theme(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Theme introuvable : {theme_id}")))?;

        if existing.guild_id.as_str() != guild_id {
            return Err(DomainError::ValidationError(
                "Ce theme n'appartient pas a ce serveur".to_string(),
            ));
        }

        self.repo.delete_theme(id).await
    }
}
