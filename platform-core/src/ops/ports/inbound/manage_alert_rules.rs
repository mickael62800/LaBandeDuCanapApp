//! Port inbound : gestion des règles d'alerte de supervision. Le handler HTTP
//! ne fait que RBAC (superadmin) + mapping DTO ; validation et persistance
//! vivent derrière ce port.

use async_trait::async_trait;

use crate::ops::domain::entities::alert_rule::{AlertRule, AlertRuleUpdate};
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait ManageAlertRulesUseCase: Send + Sync {
    async fn list(&self) -> Result<Vec<AlertRule>, DomainError>;

    /// Valide les invariants (sévérité, cooldown ≥ 60) puis met à jour.
    /// `NotFound` si la règle n'existe pas.
    async fn update(&self, id: &str, update: AlertRuleUpdate) -> Result<AlertRule, DomainError>;
}
