//! Port outbound : persistance de l'audit serveur (`server_events`).
//! Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;

use crate::domain::entities::ops::server_event::{ServerEvent, ServerEventFilter};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait ServerEventRepository: Send + Sync {
    /// Insere une row d'event serveur.
    async fn record(
        &self,
        actor: &str,
        actor_name: Option<&str>,
        action: &str,
        target: Option<&str>,
        severity: &str,
        details: serde_json::Value,
    ) -> Result<(), DomainError>;

    /// Liste les events serveur selon les filtres (deja bornes).
    async fn list(&self, filter: &ServerEventFilter) -> Result<Vec<ServerEvent>, DomainError>;
}
