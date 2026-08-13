use async_trait::async_trait;

use crate::sentinel::domain::entities::community::guild_member::GuildMember;
use crate::sentinel::domain::entities::community::guild_member::MemberSummary;
use crate::sentinel::domain::entities::community::milestone::JoinAnniversary;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

pub struct SyncMembersCommand {
    pub guild_id: GuildId,
    pub members: Vec<GuildMember>,
}

pub struct RegisterMemberCommand {
    pub member: GuildMember,
}

pub struct UpdateMemberCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}

#[async_trait]
pub trait ManageMembersUseCase: Send + Sync {
    async fn list_members(&self, guild_id: &str) -> Result<Vec<GuildMember>, DomainError>;
    async fn get_member(&self, guild_id: &str, user_id: &str) -> Result<GuildMember, DomainError>;
    async fn get_member_summary(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<MemberSummary, DomainError>;
    async fn sync_members(&self, cmd: SyncMembersCommand) -> Result<u64, DomainError>;
    async fn register_member(&self, cmd: RegisterMemberCommand) -> Result<(), DomainError>;
    async fn remove_member(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn update_member(&self, cmd: UpdateMemberCommand) -> Result<(), DomainError>;

    /// Reinitialise TOUTES les donnees de moderation d'un membre (transaction
    /// atomique). Renvoie, par table, la cle de reponse et le nombre de lignes
    /// supprimees.
    async fn reset_member(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<(&'static str, u64)>, DomainError>;

    /// Marque un membre comme parti (idempotent) + reset wallet. Renvoie le
    /// nombre de lignes guild_members MAJ.
    async fn leave_member(&self, guild_id: &str, user_id: &str) -> Result<u64, DomainError>;

    /// Marque un membre comme revenu. Renvoie le nombre de lignes MAJ.
    async fn rejoin_member(&self, guild_id: &str, user_id: &str) -> Result<u64, DomainError>;

    /// Anniversaires d'arrivee a venir dans les `days` prochains jours.
    /// Sert la section « anniversaires » de l'espace membre.
    async fn upcoming_anniversaries(
        &self,
        guild_id: &str,
        days: i32,
    ) -> Result<Vec<JoinAnniversary>, DomainError>;

    /// Membres arrives dans les `days` derniers jours.
    async fn recent_joins(
        &self,
        guild_id: &str,
        days: i32,
        limit: i64,
    ) -> Result<Vec<GuildMember>, DomainError>;
}
