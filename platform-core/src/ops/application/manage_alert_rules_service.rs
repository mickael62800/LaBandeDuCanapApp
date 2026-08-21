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

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeAlertRuleRepo;
    #[async_trait]
    impl AlertRuleRepository for FakeAlertRuleRepo {
        async fn list(&self) -> Result<Vec<AlertRule>, DomainError> {
            Ok(vec![])
        }

        async fn update(
            &self,
            _id: &str,
            update: &AlertRuleUpdate,
        ) -> Result<Option<AlertRule>, DomainError> {
            Ok(Some(AlertRule {
                id: "rule1".into(),
                label: "Test Rule".into(),
                metric: "cpu_usage".into(),
                comparator: update.threshold.map(|_| "gt".into()).unwrap_or_default(),
                threshold: update.threshold,
                enabled: update.enabled.unwrap_or(true),
                severity: update.severity.clone().unwrap_or_default(),
                cooldown_secs: update.cooldown_secs.unwrap_or(60),
            }))
        }
    }

    #[test]
    fn service_can_be_created() {
        let _service = ManageAlertRulesService::new(Arc::new(FakeAlertRuleRepo));
    }

    #[tokio::test]
    async fn list_delegates_to_repo() {
        let service = ManageAlertRulesService::new(Arc::new(FakeAlertRuleRepo));
        let result = service.list().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn update_validates_before_delegating() {
        let service = ManageAlertRulesService::new(Arc::new(FakeAlertRuleRepo));
        let update = AlertRuleUpdate {
            enabled: None,
            threshold: None,
            severity: Some("invalid_severity".into()),
            cooldown_secs: None,
        };
        let result = service.update("rule1", update).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_returns_not_found_when_rule_missing() {
        struct NotFoundRepo;
        #[async_trait]
        impl AlertRuleRepository for NotFoundRepo {
            async fn list(&self) -> Result<Vec<AlertRule>, DomainError> {
                Ok(vec![])
            }

            async fn update(
                &self,
                _id: &str,
                _update: &AlertRuleUpdate,
            ) -> Result<Option<AlertRule>, DomainError> {
                Ok(None)
            }
        }

        let service = ManageAlertRulesService::new(Arc::new(NotFoundRepo));
        let update = AlertRuleUpdate {
            enabled: None,
            threshold: None,
            severity: None,
            cooldown_secs: None,
        };
        let result = service.update("missing", update).await;
        assert!(matches!(result, Err(DomainError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_applies_valid_changes() {
        let service = ManageAlertRulesService::new(Arc::new(FakeAlertRuleRepo));
        let update = AlertRuleUpdate {
            enabled: Some(false),
            threshold: Some(75.0),
            severity: Some("warning".into()),
            cooldown_secs: Some(120),
        };
        let result = service.update("rule1", update).await;
        assert!(result.is_ok());
        let rule = result.unwrap();
        assert_eq!(rule.severity, "warning");
    }
}
