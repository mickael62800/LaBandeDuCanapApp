use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::voice_channel::VoiceChannel;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelBan;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelCoAdmin;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelInviteLink;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelPreset;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelTheme;
use crate::sentinel::domain::entities::community::voice_channel::VoiceChannelWhitelistEntry;
use crate::sentinel::domain::errors::DomainError;

/// Gestion du cycle de vie des salons vocaux (CRUD + mises a jour d'attributs).
#[async_trait]
pub trait VoiceChannelStore: Send + Sync {
    async fn find_all(&self) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn find_all_by_guild(&self, guild_id: &str) -> Result<Vec<VoiceChannel>, DomainError>;
    /// Historique : salons `channel_status = 'closed'` d'une guild, tries
    /// par `closed_at` DESC, limites a `limit` entrees.
    async fn find_closed_by_guild(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<VoiceChannel>, DomainError>;
    async fn find_by_channel_id(
        &self,
        channel_id: &str,
    ) -> Result<Option<VoiceChannel>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<VoiceChannel>, DomainError>;
    async fn save(&self, channel: &VoiceChannel) -> Result<(), DomainError>;
    async fn close(&self, id: Uuid) -> Result<(), DomainError>;
    async fn close_by_channel_id(&self, channel_id: &str) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    /// Hard-delete d'un salon archive (`channel_status = 'closed'`) via son
    /// `channel_id`. Retourne le nombre de lignes supprimees (0 si le salon est
    /// introuvable ou toujours ouvert).
    async fn hard_delete_closed_by_channel_id(&self, channel_id: &str) -> Result<u64, DomainError>;
    /// Hard-delete de tous les salons fermes d'une guild. Retourne le nombre de
    /// lignes supprimees.
    async fn hard_delete_closed_by_guild(&self, guild_id: &str) -> Result<u64, DomainError>;
    async fn update_visibility(&self, id: Uuid, visibility: &str) -> Result<(), DomainError>;
    async fn update_locked(&self, id: Uuid, locked: bool) -> Result<(), DomainError>;
    async fn update_queue_enabled(&self, id: Uuid, queue_enabled: bool) -> Result<(), DomainError>;
    async fn update_name(&self, id: Uuid, name: &str) -> Result<(), DomainError>;
    async fn update_status(&self, id: Uuid, status: Option<&str>) -> Result<(), DomainError>;
    async fn update_member_limit(&self, id: Uuid, limit: Option<i32>) -> Result<(), DomainError>;
    async fn update_owner(
        &self,
        id: Uuid,
        owner_id: &str,
        owner_name: &str,
    ) -> Result<(), DomainError>;
    async fn update_queue_channel(
        &self,
        id: Uuid,
        queue_channel_id: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn update_stage(&self, id: Uuid, stage_enabled: bool) -> Result<(), DomainError>;
}

/// Gestion des co-administrateurs d'un salon vocal.
#[async_trait]
pub trait VoiceCoAdminStore: Send + Sync {
    async fn find_co_admins(
        &self,
        voice_channel_id: Uuid,
    ) -> Result<Vec<VoiceChannelCoAdmin>, DomainError>;
    async fn add_co_admin(&self, co_admin: &VoiceChannelCoAdmin) -> Result<(), DomainError>;
    async fn remove_co_admin(
        &self,
        voice_channel_id: Uuid,
        user_id: &str,
    ) -> Result<(), DomainError>;
}

/// Gestion des listes blanches par proprietaire.
#[async_trait]
pub trait VoiceWhitelistStore: Send + Sync {
    async fn find_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Vec<VoiceChannelWhitelistEntry>, DomainError>;
    async fn add_to_whitelist(&self, entry: &VoiceChannelWhitelistEntry)
        -> Result<(), DomainError>;
    async fn remove_from_whitelist(
        &self,
        guild_id: &str,
        owner_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError>;
}

/// Gestion des presets de configuration par proprietaire.
#[async_trait]
pub trait VoicePresetStore: Send + Sync {
    async fn find_preset(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Option<VoiceChannelPreset>, DomainError>;
    async fn upsert_preset(&self, preset: &VoiceChannelPreset) -> Result<(), DomainError>;
}

/// Gestion des bannissements de salon vocal.
///
/// Les bans sont cles par (guild_id, owner_id, user_id) — comme la whitelist —
/// afin de survivre a la suppression/recreation du salon temporaire (issue #2).
#[async_trait]
pub trait VoiceBanStore: Send + Sync {
    /// Tous les bans memorises pour ce proprietaire dans cette guild (actifs
    /// ou expires), tries du plus recent au plus ancien.
    async fn find_bans_for_owner(
        &self,
        guild_id: &str,
        owner_id: &str,
    ) -> Result<Vec<VoiceChannelBan>, DomainError>;
    async fn find_active_ban(
        &self,
        guild_id: &str,
        owner_id: &str,
        user_id: &str,
    ) -> Result<Option<VoiceChannelBan>, DomainError>;
    async fn save_ban(&self, ban: &VoiceChannelBan) -> Result<(), DomainError>;
    async fn remove_ban(
        &self,
        guild_id: &str,
        owner_id: &str,
        user_id: &str,
    ) -> Result<(), DomainError>;
    async fn cleanup_expired_bans(&self) -> Result<u64, DomainError>;
}

/// Gestion des liens d'invitation vers les salons vocaux.
#[async_trait]
pub trait VoiceInviteStore: Send + Sync {
    async fn find_invite_links(
        &self,
        voice_channel_id: Uuid,
    ) -> Result<Vec<VoiceChannelInviteLink>, DomainError>;
    async fn find_invite_by_code(
        &self,
        code: &str,
    ) -> Result<Option<VoiceChannelInviteLink>, DomainError>;
    async fn save_invite_link(&self, link: &VoiceChannelInviteLink) -> Result<(), DomainError>;
    async fn increment_invite_uses(&self, id: Uuid) -> Result<bool, DomainError>;
    async fn revoke_invite_link(&self, id: Uuid) -> Result<(), DomainError>;
}

/// Gestion des themes de salon vocal d'une guild.
#[async_trait]
pub trait VoiceThemeStore: Send + Sync {
    async fn find_themes(&self, guild_id: &str) -> Result<Vec<VoiceChannelTheme>, DomainError>;
    async fn find_theme(&self, id: Uuid) -> Result<Option<VoiceChannelTheme>, DomainError>;
    async fn save_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError>;
    async fn update_theme(&self, theme: &VoiceChannelTheme) -> Result<(), DomainError>;
    async fn delete_theme(&self, id: Uuid) -> Result<(), DomainError>;
    async fn clear_default_themes(&self, guild_id: &str) -> Result<(), DomainError>;
}

/// Supertrait marqueur : regroupe l'ensemble des stores du domaine voix.
/// Permet de continuer a manipuler un `dyn VoiceChannelRepository` unique.
pub trait VoiceChannelRepository:
    VoiceChannelStore
    + VoiceCoAdminStore
    + VoiceWhitelistStore
    + VoicePresetStore
    + VoiceBanStore
    + VoiceInviteStore
    + VoiceThemeStore
{
}

impl<T> VoiceChannelRepository for T where
    T: VoiceChannelStore
        + VoiceCoAdminStore
        + VoiceWhitelistStore
        + VoicePresetStore
        + VoiceBanStore
        + VoiceInviteStore
        + VoiceThemeStore
{
}
