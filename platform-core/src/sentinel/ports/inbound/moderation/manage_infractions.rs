use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::infraction::Infraction;
use crate::sentinel::domain::errors::DomainError;

pub struct InfractionFilters {
    pub user_id: Option<String>,
    pub action: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

/// Compteurs d'infractions d'un membre, par nature d'action.
///
/// `total` couvre toutes les natures, y compris celles qui n'ont pas de
/// compteur dedie ici.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct UserInfractionCounts {
    pub warns: u32,
    pub deletes: u32,
    pub mutes: u32,
    pub bans: u32,
    pub total: u32,
}

#[async_trait]
pub trait ManageInfractionsUseCase: Send + Sync {
    /// Compteurs d'un membre, agreges en base (pas de rapatriement du journal).
    async fn count_user_infractions(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserInfractionCounts, DomainError>;

    async fn list_infractions(
        &self,
        guild_id: &str,
        filters: InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError>;

    async fn list_all_infractions(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Infraction>, DomainError>;

    async fn count_today(&self) -> Result<u64, DomainError>;

    async fn find_by_id(&self, id: &str) -> Result<Option<Infraction>, DomainError>;

    async fn delete_infraction(&self, id: &str) -> Result<bool, DomainError>;

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;
}
