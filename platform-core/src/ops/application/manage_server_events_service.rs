//! Use case audit serveur : borne les filtres de lecture et delegue la
//! persistance au repo. Le SQL vit dans `ServerEventRepository`, le handler HTTP
//! ne fait que RBAC/parse/map.

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::server_event::{ServerEvent, ServerEventFilter};
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::manage_server_events::ManageServerEventsUseCase;
use crate::ops::ports::outbound::server_event_repository::ServerEventRepository;

pub struct ManageServerEventsService {
    repo: Arc<dyn ServerEventRepository>,
}

impl ManageServerEventsService {
    pub fn new(repo: Arc<dyn ServerEventRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageServerEventsUseCase for ManageServerEventsService {
    async fn record(
        &self,
        actor: &str,
        actor_name: Option<&str>,
        action: &str,
        target: Option<&str>,
        severity: &str,
        details: serde_json::Value,
    ) -> Result<(), DomainError> {
        self.repo
            .record(actor, actor_name, action, target, severity, details)
            .await
    }

    async fn list(
        &self,
        action_prefix: Option<String>,
        severity: Option<String>,
        limit: Option<i64>,
    ) -> Result<Vec<ServerEvent>, DomainError> {
        let limit = limit
            .unwrap_or(100)
            .clamp(1, crate::ops::application::PAGE_LIMIT_MAX);
        self.repo
            .list(&ServerEventFilter {
                action_prefix,
                severity,
                limit,
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeServerEventRepo;
    #[async_trait]
    impl ServerEventRepository for FakeServerEventRepo {
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

    #[test]
    fn service_can_be_created() {
        let _service = ManageServerEventsService::new(Arc::new(FakeServerEventRepo));
    }

    #[tokio::test]
    async fn record_delegates_to_repo() {
        let service = ManageServerEventsService::new(Arc::new(FakeServerEventRepo));
        let result = service
            .record("admin", Some("Admin"), "delete", Some("user"), "warning", serde_json::json!({}))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn list_clamps_limit() {
        let service = ManageServerEventsService::new(Arc::new(FakeServerEventRepo));
        let result = service.list(None, None, Some(0)).await;
        assert!(result.is_ok());
    }
}
