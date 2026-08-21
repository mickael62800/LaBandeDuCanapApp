//! Port outbound : persistance de l'audit serveur (`server_events`).
//! Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;

use crate::ops::domain::entities::server_event::{NewServerEvent, ServerEvent, ServerEventFilter};
use crate::ops::domain::errors::DomainError;

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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestServerEventRepo;
    #[async_trait]
    impl ServerEventRepository for TestServerEventRepo {
        async fn record(
            &self,
            _actor: &str,
            _actor_name: Option<&str>,
            _action: &str,
            _target: Option<&str>,
            _severity: &str,
            _details: serde_json::Value,
        ) -> Result<(), DomainError> {
            Ok(())
        }

        async fn list(&self, _filter: &ServerEventFilter) -> Result<Vec<ServerEvent>, DomainError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn record_batch_default_implementation() {
        let repo = TestServerEventRepo;
        let events = vec![NewServerEvent {
            actor: "admin".into(),
            actor_name: Some("Admin".into()),
            action: "delete".into(),
            target: Some("user".into()),
            severity: "warning".into(),
            details: serde_json::json!({}),
        }];
        let result = repo.record_batch(&events).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn record_batch_with_multiple_events() {
        let repo = TestServerEventRepo;
        let events = vec![
            NewServerEvent {
                actor: "admin".into(),
                actor_name: Some("Admin".into()),
                action: "delete".into(),
                target: Some("user1".into()),
                severity: "info".into(),
                details: serde_json::json!({}),
            },
            NewServerEvent {
                actor: "admin".into(),
                actor_name: Some("Admin".into()),
                action: "create".into(),
                target: Some("user2".into()),
                severity: "info".into(),
                details: serde_json::json!({}),
            },
        ];
        let result = repo.record_batch(&events).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn record_with_all_fields() {
        let repo = TestServerEventRepo;
        let result = repo
            .record(
                "actor123",
                Some("Actor Name"),
                "create_user",
                Some("target_id"),
                "info",
                serde_json::json!({"extra": "data"}),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn record_without_optional_fields() {
        let repo = TestServerEventRepo;
        let result = repo
            .record(
                "system",
                None,
                "auto_cleanup",
                None,
                "debug",
                serde_json::json!({}),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_with_filter() {
        let repo = TestServerEventRepo;
        let filter = ServerEventFilter {
            action_prefix: Some("docker".into()),
            severity: Some("warning".into()),
            limit: 50,
        };
        let result = repo.list(&filter).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn record_batch_empty() {
        let repo = TestServerEventRepo;
        let result = repo.record_batch(&[]).await;
        assert!(result.is_ok());
    }
}
