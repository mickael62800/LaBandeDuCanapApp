use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct ReviewEntry {
    pub id: Uuid,
    pub action_id: Uuid,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    pub reason: Option<String>,
    pub status: String,
    pub reviewer_id: Option<String>,
    pub reviewer_name: Option<String>,
    pub reviewer_notes: Option<String>,
    pub added_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    // Enrichi via JOIN
    pub action_type: Option<String>,
    pub target_name: Option<String>,
    pub action_reason: Option<String>,
}

#[async_trait]
pub trait ReviewRepository: Send + Sync {
    async fn add(
        &self,
        action_id: Uuid,
        guild_id: &str,
        added_by: &str,
        added_by_name: &str,
        reason: Option<&str>,
    ) -> Result<ReviewEntry, DomainError>;
    async fn list_pending(&self, guild_id: &str) -> Result<Vec<ReviewEntry>, DomainError>;
    async fn resolve(
        &self,
        review_id: Uuid,
        reviewer_id: &str,
        reviewer_name: &str,
        notes: Option<&str>,
        status: &str,
    ) -> Result<bool, DomainError>;
    async fn get_guild_id(&self, review_id: Uuid) -> Result<Option<String>, DomainError>;
}
