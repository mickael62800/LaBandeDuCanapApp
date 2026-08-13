use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::RoleId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TempRole {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub role_id: RoleId,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait TempRoleRepository: Send + Sync {
    async fn create(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) -> Result<(), DomainError>;
    async fn list_active(&self, guild_id: &str) -> Result<Vec<TempRole>, DomainError>;
    async fn delete(&self, guild_id: &str, user_id: &str, role_id: &str)
        -> Result<(), DomainError>;
}
