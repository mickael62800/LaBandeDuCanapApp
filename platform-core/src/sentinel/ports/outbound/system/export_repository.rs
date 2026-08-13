//! Port pour les queries d'export (CSV/JSON dashboard).
//! L'adapter Postgres execute les SELECT, le service applicatif serialise.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone, serde::Serialize)]
pub struct InfractionExport {
    pub id: Uuid,
    pub guild_id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub message_id: String,
    pub content: String,
    pub score: f64,
    pub action: String,
    pub reason: String,
    pub duration: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditLogExport {
    pub id: Uuid,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModerationActionExport {
    pub id: Uuid,
    pub guild_id: String,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub duration: Option<i64>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait ExportRepository: Send + Sync {
    async fn fetch_infractions(
        &self,
        guild_id: &str,
        max_rows: i64,
    ) -> Result<Vec<InfractionExport>, DomainError>;
    async fn fetch_audit_logs(
        &self,
        guild_id: &str,
        max_rows: i64,
    ) -> Result<Vec<AuditLogExport>, DomainError>;
    async fn fetch_moderation_actions(
        &self,
        guild_id: &str,
        max_rows: i64,
    ) -> Result<Vec<ModerationActionExport>, DomainError>;
}
