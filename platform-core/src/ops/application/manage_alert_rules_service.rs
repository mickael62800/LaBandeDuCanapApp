//! Use case des règles d'alerte : invariants métier + délégation au repo.

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::alert_rule::{AlertRule, AlertRuleUpdate};
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::manage_alert_rules::ManageAlertRulesUseCase;
use crate::ops::ports::outbound::alert_rule_repository::AlertRuleRepository;

pub struct ManageAlertRulesService {
    repo: Arc<dyn AlertRuleRepository>,
}

impl ManageAlertRulesService {
    pub fn new(repo: Arc<dyn AlertRuleRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageAlertRulesUseCase for ManageAlertRulesService {
    async fn list(&self) -> Result<Vec<AlertRule>, DomainError> {
        self.repo.list().await
    }

    async fn update(&self, id: &str, update: AlertRuleUpdate) -> Result<AlertRule, DomainError> {
        update.validate()?;
        self.repo
            .update(id, &update)
            .await?
            .ok_or_else(|| DomainError::NotFound("regle d'alerte inconnue".into()))
    }
}
