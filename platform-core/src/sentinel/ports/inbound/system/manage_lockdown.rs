//! Port inbound : gestion du lockdown de securite (`security_lockdown_active`).
//! Le handler HTTP ne fait que parser/RBAC/mapper ; la regle metier (calcul de
//! l'expiration) vit dans le service, le SQL dans `LockdownRepository`.

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageLockdownUseCase: Send + Sync {
    /// (Re)active un lockdown pour une guild. `saved_states` decrit les
    /// overwrites originaux par salon (a restaurer a l'expiration). Une
    /// re-activation reset le timer + les states (idempotent).
    async fn activate(
        &self,
        guild_id: &str,
        saved_states: serde_json::Value,
        duration_secs: i64,
    ) -> Result<(), DomainError>;

    /// Retire le lockdown d'une guild (deactivation manuelle ou worker). Idempotent.
    async fn deactivate(&self, guild_id: &str) -> Result<(), DomainError>;
}
