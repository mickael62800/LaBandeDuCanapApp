//! Port outbound : persistance de l'audit serveur (`server_events`).
//! Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;

use crate::domain::entities::server_event::{NewServerEvent, ServerEvent, ServerEventFilter};
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

    /// Insere plusieurs events en une fois (un seul aller-retour cote adapter).
    ///
    /// Le monitor Docker peut produire de nombreux changements dans un meme
    /// relevé (recreation massive de conteneurs) : les inserer un a un imposait
    /// autant d'allers-retours SQL. L'impl par defaut boucle sur `record` pour
    /// ne rien casser ; l'adapter Postgres l'ecrase par un insert groupe.
    async fn record_batch(&self, events: &[NewServerEvent]) -> Result<(), DomainError> {
        for event in events {
            self.record(
                &event.actor,
                event.actor_name.as_deref(),
                &event.action,
                event.target.as_deref(),
                &event.severity,
                event.details.clone(),
            )
            .await?;
        }
        Ok(())
    }

    /// Liste les events serveur selon les filtres (deja bornes).
    async fn list(&self, filter: &ServerEventFilter) -> Result<Vec<ServerEvent>, DomainError>;
}
