use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GameAuditRepository: Send + Sync {
    async fn log(
        &self,
        guild_id: &str,
        server_id: Option<Uuid>,
        actor_user_id: Option<&str>,
        action: GameAuditAction,
        details: serde_json::Value,
    ) -> Result<(), DomainError>;

    async fn list_for_server(
        &self,
        server_id: Uuid,
        limit: i64,
    ) -> Result<Vec<GameAuditEntry>, DomainError>;

    async fn list_for_guild(
        &self,
        guild_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<GameAuditEntry>, DomainError>;
}
