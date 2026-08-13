//! Port outbound : persistance des règles d'alerte (`alert_rules`).

use async_trait::async_trait;

use crate::ops::domain::entities::alert_rule::{AlertRule, AlertRuleUpdate};
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait AlertRuleRepository: Send + Sync {
    /// Liste toutes les règles (actives ou non), triées par id.
    async fn list(&self) -> Result<Vec<AlertRule>, DomainError>;

    /// Met à jour les champs fournis (COALESCE) ; `None` = règle inconnue.
    async fn update(
        &self,
        id: &str,
        update: &AlertRuleUpdate,
    ) -> Result<Option<AlertRule>, DomainError>;
}
