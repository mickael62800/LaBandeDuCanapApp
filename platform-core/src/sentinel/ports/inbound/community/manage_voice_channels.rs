use async_trait::async_trait;

use crate::sentinel::domain::entities::community::voice_channel::VoiceChannel;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelConfig;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelDetail;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelPreset;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

pub struct CreateVoiceChannelCommand {
    pub guild_id: GuildId,
    pub owner_id: String,
    pub owner_name: String,
    pub channel_id: ChannelId,
    pub text_channel_id: Option<String>,
    pub members_channel_id: Option<String>,
    pub queue_channel_id: Option<String>,
    pub category_id: Option<String>,
    pub channel_name: String,
    pub kind: String,
    pub visibility: String,
    pub queue_enabled: bool,
    pub stage_enabled: bool,
}

pub struct UpdateVoiceChannelCommand {
    pub channel_id: ChannelId,
    pub visibility: Option<String>,
    pub locked: Option<bool>,
    pub queue_enabled: Option<bool>,
    pub name: Option<String>,
    pub status: Option<String>,
    pub member_limit: Option<Option<i32>>,
    pub queue_channel_id: Option<Option<String>>,
    pub stage_enabled: Option<bool>,
}

pub struct TransferOwnershipCommand {
    pub channel_id: ChannelId,
    pub new_owner_id: String,
    pub new_owner_name: String,
}

pub struct ManageCoAdminCommand {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub user_name: String,
}

pub struct ManageWhitelistCommand {
    pub guild_id: GuildId,
    pub owner_id: String,
    pub target_id: String,
    pub target_name: String,
}

pub struct SavePresetCommand {
    pub guild_id: GuildId,
    pub owner_id: String,
    pub channel_name: Option<String>,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
}

pub struct BanFromChannelCommand {
    pub channel_id: ChannelId,
    pub user_id: UserId,
    pub user_name: String,
    pub banned_by: String,
    pub reason: Option<String>,
    pub duration_secs: Option<i64>,
}

pub struct CreateInviteLinkCommand {
    pub channel_id: ChannelId,
    pub created_by: String,
    pub created_by_name: String,
    pub duration_secs: Option<i64>,
    pub max_uses: Option<i32>,
}

pub struct UseInviteLinkCommand {
    pub code: String,
    pub user_id: UserId,
    pub user_name: String,
}

pub struct CreateThemeCommand {
    pub guild_id: GuildId,
    pub name: String,
    pub emoji: Option<String>,
    pub channel_name_template: String,
    pub member_limit: Option<i32>,
    pub visibility: String,
    pub locked: bool,
    pub queue_enabled: bool,
    pub bitrate: Option<i32>,
    pub slowmode_secs: Option<i32>,
    pub stage_enabled: bool,
    pub is_default: bool,
    pub sort_order: i32,
}

#[async_trait]
pub trait ManageVoiceChannelsUseCase: Send + Sync {
    async fn list_all_channels(&self) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn list_channels(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError>;
    /// Historique : salons fermes d'une guild, limites a `limit`.
    async fn list_history_channels(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn get_channel_detail(&self, channel_id: &str)
        -> Result<VoiceChannelDetail, DomainError>;
    async fn create_channel(
        &self,
        cmd: CreateVoiceChannelCommand,
    ) -> Result<VoiceChannel, DomainError>;
    async fn close_channel(&self, channel_id: &str) -> Result<(), DomainError>;
    async fn delete_channel(&self, channel_id: &str) -> Result<(), DomainError>;
    /// Resout le `guild_id` associe a un salon (via son `channel_id`). Renvoie
    /// `None` si aucun salon ne correspond. Utilise par les gardes RBAC du
    /// handler pour scoper l'autorisation a la guilde du salon.
    async fn find_guild_id(&self, channel_id: &str) -> Result<Option<String>, DomainError>;
    /// Purge (hard-delete) un salon archive via son `channel_id`. Renvoie `true`
    /// si une ligne a ete supprimee, `false` si le salon est introuvable ou
    /// encore ouvert.
    async fn purge_channel(&self, channel_id: &str) -> Result<bool, DomainError>;
    /// Purge (hard-delete) tous les salons fermes d'une guild. Renvoie le nombre
    /// de salons supprimes.
    async fn purge_history(&self, guild_id: &str) -> Result<u64, DomainError>;
    async fn update_channel(&self, cmd: UpdateVoiceChannelCommand) -> Result<(), DomainError>;
    async fn transfer_ownership(&self, cmd: TransferOwnershipCommand) -> Result<(), DomainError>;

    // Co-admins
    async fn add_co_admin(&self, cmd: ManageCoAdminCommand) -> Result<(), DomainError>;
    async fn remove_co_admin(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError>;

    // Whitelist
    async fn get_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError>;
    async fn add_to_whitelist(&self, cmd: ManageWhitelistCommand) -> Result<(), DomainError>;
    async fn remove_from_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError>;

    // Presets par proprietaire
    async fn get_preset(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<VoiceChannelPreset>, DomainError>;
    async fn save_preset(&self, cmd: SavePresetCommand) -> Result<(), DomainError>;

    // Bans
    async fn ban_from_channel(&self, cmd: BanFromChannelCommand) -> Result<(), DomainError>;
    async fn unban_from_channel(&self, channel_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn is_banned(&self, channel_id: &str, user_id: &str) -> Result<bool, DomainError>;
    /// Bans memorises pour un proprietaire (guild + owner). Utilise pour
    /// re-appliquer les bans a la recreation d'un salon temporaire.
    async fn list_owner_bans(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::community::voice_channel::VoiceChannelBan>,
        DomainError,
    >;

    // Invite Links
    async fn create_invite_link(
        &self,
        cmd: CreateInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError>;
    async fn list_invite_links(
        &self,
        channel_id: &str,
    ) -> Result<Vec<VoiceChannelInviteLink>, DomainError>;
    async fn use_invite_link(
        &self,
        cmd: UseInviteLinkCommand,
    ) -> Result<VoiceChannelInviteLink, DomainError>;
    async fn revoke_invite_link(&self, channel_id: &str, link_id: &str) -> Result<(), DomainError>;

    // Config voice-bot par guild
    async fn get_voice_config(&self, guild_id: &str) -> Result<VoiceChannelConfig, DomainError>;

    // Themes
    async fn list_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError>;
    async fn create_theme(&self, cmd: CreateThemeCommand)
        -> Result<VoiceChannelTheme, DomainError>;
    async fn update_theme(
        &self,
        theme_id: &str,
        cmd: CreateThemeCommand,
    ) -> Result<VoiceChannelTheme, DomainError>;
    async fn delete_theme(&self, guild_id: &str, theme_id: &str) -> Result<(), DomainError>;
}
