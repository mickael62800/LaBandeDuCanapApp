//! Port inbound : gestion du slowmode de securite manuel
//! (`security_slowmode_active`). Le handler HTTP ne fait que parser/RBAC/mapper ;
//! la regle metier (calcul de l'expiration) vit dans le service, le SQL dans
//! `SlowmodeRepository`.

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageSlowmodeUseCase: Send + Sync {
    /// (Re)active un slowmode pour une guild. `previous_states` decrit les rates
    /// d'origine par salon (a restaurer a l'expiration), `imposed_rate` le rate
    /// pose par le raid. Une re-activation reset le timer + les states (idempotent).
    async fn activate(
        &self,
        guild_id: &str,
        previous_states: serde_json::Value,
        duration_secs: i64,
        imposed_rate: i32,
    ) -> Result<(), DomainError>;

    /// Retire le slowmode d'une guild (deactivation manuelle ou worker). Idempotent.
    async fn deactivate(&self, guild_id: &str) -> Result<(), DomainError>;
}
