//! Port inbound : audit serveur (`server_events`). Le handler HTTP ne fait que
//! RBAC/parse/map ; le bornage des filtres vit dans le service, le SQL dans le
//! `ServerEventRepository`.

use async_trait::async_trait;

use crate::domain::entities::ops::server_event::ServerEvent;
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ManageServerEventsUseCase: Send + Sync {
    /// Enregistre un event serveur (best-effort cote appelant).
    async fn record(
        &self,
        actor: &str,
        actor_name: Option<&str>,
        action: &str,
        target: Option<&str>,
        severity: &str,
        details: serde_json::Value,
    ) -> Result<(), DomainError>;

    /// Liste les events serveur les plus recents. `limit` est borne [1, 500].
    async fn list(
        &self,
        action_prefix: Option<String>,
        severity: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<ServerEvent>, DomainError>;
}
