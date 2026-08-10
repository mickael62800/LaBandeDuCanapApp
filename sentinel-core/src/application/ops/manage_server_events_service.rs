//! Use case audit serveur : borne les filtres de lecture et delegue la
//! persistance au repo. Le SQL vit dans `ServerEventRepository`, le handler HTTP
//! ne fait que RBAC/parse/map.

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::ops::server_event::{ServerEvent, ServerEventFilter};
use crate::domain::errors::DomainError;
use crate::ports::inbound::ops::manage_server_events::ManageServerEventsUseCase;
use crate::ports::outbound::ops::server_event_repository::ServerEventRepository;

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
            .clamp(1, crate::application::validation::PAGE_LIMIT_MAX);
        self.repo
            .list(&ServerEventFilter {
                action_prefix,
                severity,
                limit,
            })
            .await
    }
}
