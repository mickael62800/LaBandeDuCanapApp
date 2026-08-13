use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::voice_channel::VoiceChannel;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelConfig;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelDetail;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelPreset;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::CreateThemeCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::CreateVoiceChannelCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::ManageVoiceChannelsUseCase;
use crate::sentinel::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::SavePresetCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use crate::sentinel::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;
use crate::sentinel::ports::outbound::community::voice_channel_repository::VoiceChannelRepository;
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;
mod access_control;
mod co_admin;
mod config;
mod crud;
mod invite;
mod theme;

pub(crate) const CHANNELS_LIST_TTL: u64 = 60;
pub(crate) const CHANNEL_DETAIL_TTL: u64 = 300;

pub struct ManageVoiceChannelsService {
    pub(super) repo: Arc<dyn VoiceChannelRepository>,
    pub(super) cache: Arc<dyn CachePort>,
    pub(super) bot_config_repo: Arc<dyn BotConfigRepository>,
}

impl ManageVoiceChannelsService {
    pub fn new(
        repo: Arc<dyn VoiceChannelRepository>,
        cache: Arc<dyn CachePort>,
        bot_config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self {
            repo,
            cache,
            bot_config_repo,
        }
    }

    pub(super) fn generate_code() -> String {
        use rand::Rng;
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>()
            .to_uppercase()
    }

    /// Genere un code unique avec retry en cas de collision (UNIQUE constraint en DB).
    pub(super) async fn generate_unique_code(&self) -> Result<String, DomainError> {
        for _ in 0..5 {
            let code = Self::generate_code();
            // Verifier si le code existe deja
            if self.repo.find_invite_by_code(&code).await?.is_none() {
                return Ok(code);
            }
        }
        Err(DomainError::Internal(
            "Impossible de generer un code unique apres 5 tentatives".to_string(),
        ))
    }

    pub(super) fn validate_theme(cmd: &CreateThemeCommand) -> Result<(), DomainError> {
        if cmd.name.trim().is_empty() {
            return Err(DomainError::ValidationError(
                "Le nom du theme est obligatoire".to_string(),
            ));
        }
        if cmd.name.len() > 100 {
            return Err(DomainError::ValidationError(
                "Le nom du theme ne peut pas depasser 100 caracteres".to_string(),
            ));
        }
        if let Some(limit) = cmd.member_limit {
            if !(0..=99).contains(&limit) {
                return Err(DomainError::ValidationError(
                    "La limite de membres doit etre entre 0 et 99".to_string(),
                ));
            }
        }
        if let Some(bitrate) = cmd.bitrate {
            if !(8000..=384000).contains(&bitrate) {
                return Err(DomainError::ValidationError(
                    "Le bitrate doit etre entre 8000 et 384000".to_string(),
                ));
            }
        }
        if let Some(slowmode) = cmd.slowmode_secs {
            if !(0..=21600).contains(&slowmode) {
                return Err(DomainError::ValidationError(
                    "Le slowmode doit etre entre 0 et 21600 secondes".to_string(),
                ));
            }
        }
        match cmd.visibility.as_str() {
            "visible" | "hidden" => {}
            _ => {
                return Err(DomainError::ValidationError(
                    "La visibilite doit etre 'visible' ou 'hidden'".to_string(),
                ))
            }
        }
        Ok(())
    }

    pub(super) async fn invalidate_cache(&self, guild_id: &str, channel_id: &str) {
        if let Err(e) = self
            .cache
            .invalidate(&format!("voice_channels:{guild_id}"))
            .await
        {
            tracing::warn!(error = %e, guild_id, "Echec invalidation cache voice_channels");
        }
        if let Err(e) = self
            .cache
            .invalidate(&format!("voice_channel:{channel_id}"))
            .await
        {
            tracing::warn!(error = %e, channel_id, "Echec invalidation cache voice_channel");
        }
    }

    pub(super) async fn resolve_channel(
        &self,
        channel_id: &str,
    ) -> Result<VoiceChannel, DomainError> {
        self.repo
            .find_by_channel_id(channel_id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("Salon vocal introuvable : {channel_id}")))
    }
}

#[async_trait]
impl ManageVoiceChannelsUseCase for ManageVoiceChannelsService {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError> {
        self.list_all_channels_impl().await
    }

    async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError> {
        self.list_channels_impl(guild_id).await
    }

    async fn list_history_channels(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError> {
        self.list_history_channels_impl(guild_id, limit).await
    }

    async fn get_channel_detail(
        &self,
        channel_id: &str,
    ) -> Result<VoiceChannelDetail, DomainError> {
        self.get_channel_detail_impl(channel_id).await
    }

    async fn create_channel(
        &self,
        cmd: CreateVoiceChannelCommand,
    ) -> Result<VoiceChannel, DomainError> {
        self.create_channel_impl(cmd).await
    }

    async fn close_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        self.close_channel_impl(channel_id).await
    }

    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError> {
        self.delete_channel_impl(channel_id).await
    }

    async fn find_guild_id(&self, channel_id: &str) -> Result<Option<String>, DomainError> {
        self.find_guild_id_impl(channel_id).await
    }

    async fn purge_channel(&self, channel_id: &str) -> Result<bool, DomainError> {
        self.purge_channel_impl(channel_id).await
    }

    async fn purge_history(&self, guild_id: &str) -> Result<u64, DomainError> {
        self.purge_history_impl(guild_id).await
    }

    async fn update_channel(&self, cmd: UpdateVoiceChannelCommand) -> Result<(), DomainError> {
        self.update_channel_impl(cmd).await
    }

    async fn transfer_ownership(&self, cmd: TransferOwnershipCommand) -> Result<(), DomainError> {
        self.transfer_ownership_impl(cmd).await
    }

    async fn add_co_admin(&self, cmd: ManageCoAdminCommand) -> Result<(), DomainError> {
        self.add_co_admin_impl(cmd).await
    }

    async fn remove_co_admin(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.remove_co_admin_impl(channel_id, user_id).await
    }

    async fn get_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError> {
        self.get_whitelist_impl(guild_id, owner_id).await
    }

    async fn add_to_whitelist(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError> {
        self.add_to_whitelist_impl(cmd).await
    }

    async fn remove_from_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        self.remove_from_whitelist_impl(guild_id, owner_id, target_id)
            .await
    }

    async fn get_preset(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<VoiceChannelPreset>, DomainError> {
        self.get_preset_impl(guild_id, owner_id).await
    }

    async fn save_preset(&self, cmd: SavePresetCommand) -> Result<(), DomainError> {
        self.save_preset_impl(cmd).await
    }

    async fn ban_from_channel(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError> {
        self.ban_from_channel_impl(cmd).await
    }

    async fn unban_from_channel(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError> {
        self.unban_from_channel_impl(channel_id, user_id).await
    }

    async fn is_banned(&self, channel_id: &str, user_id: &str) -> Result<bool, DomainError> {
        self.is_banned_impl(channel_id, user_id).await
    }

    async fn list_owner_bans(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::community::voice_channel::VoiceChannelBan>,
        DomainError,
    > {
        self.list_owner_bans_impl(guild_id, owner_id).await
    }

    // ── Invite Links ──

    async fn create_invite_link(
        &self,
        cmd: CreateInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        self.create_invite_link_impl(cmd).await
    }

    async fn list_invite_links(
        &self,
        channel_id: &str,
    ) -> Result<Vec<VoiceChannelInviteLink>, DomainError> {
        self.list_invite_links_impl(channel_id).await
    }

    async fn use_invite_link(
        &self,
        cmd: UseInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError> {
        self.use_invite_link_impl(cmd).await
    }

    async fn revoke_invite_link(&self, channel_id: &str, link_id: &str) -> Result<(), DomainError> {
        self.revoke_invite_link_impl(channel_id, link_id).await
    }

    // ── Config voice-bot par guild ──

    async fn get_voice_config(&self, guild_id: &str) -> Result<VoiceChannelConfig, DomainError> {
        self.get_voice_config_impl(guild_id).await
    }

    // ── Themes ──

    async fn list_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError> {
        self.list_themes_impl(guild_id).await
    }

    async fn create_theme(
        &self,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        self.create_theme_impl(cmd).await
    }

    async fn update_theme(
        &self,
        theme_id: &str,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError> {
        self.update_theme_impl(theme_id, cmd).await
    }

    async fn delete_theme(&self, guild_id: &str, theme_id: &str) -> Result<(), DomainError> {
        self.delete_theme_impl(guild_id, theme_id).await
    }
}

#[cfg(test)]
#[path = "tests/mod_tests.rs"]
mod tests;
