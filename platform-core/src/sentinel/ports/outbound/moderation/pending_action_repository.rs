use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct PendingAction {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<i64>,
    pub status: String,
    pub reviewed_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait PendingActionRepository: Send + Sync {
    async fn create(
        &self,
        guild_id: &str,
        moderator_id: &str,
        moderator_name: &str,
        target_id: &str,
        target_name: &str,
        action_type: &str,
        reason: &str,
        gravity: Option<&str>,
        duration: Option<i64>,
    ) -> Result<Uuid, DomainError>;
    async fn list_pending(&self, guild_id: &str) -> Result<Vec<PendingAction>, DomainError>;
    async fn get_guild_id(&self, id: Uuid) -> Result<Option<String>, DomainError>;
    async fn resolve(&self, id: Uuid, status: &str, reviewed_by: &str) -> Result<(), DomainError>;
}
