//! Port inbound : gestion des quarantaines de securite. Le handler HTTP ne fait
//! que parser/RBAC/mapper ; la regle metier (delai avant kick) vit dans le
//! service, le SQL dans `QuarantineRepository`.

use async_trait::async_trait;

use crate::sentinel::domain::entities::system::quarantine::ActiveQuarantine;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageQuarantineUseCase: Send + Sync {
    /// Met (ou remet) un membre en quarantaine. `timeout_secs` est le delai
    /// avant kick automatique ; une re-quarantaine reset le timer (idempotent).
    async fn quarantine_user(
        &self,
        guild_id: &str,
        user_id: &str,
        timeout_secs: i64,
    ) -> Result<(), DomainError>;

    /// Liste les quarantaines encore actives (non expirees).
    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError>;

    /// Leve la quarantaine d'un membre (captcha valide ou retrait admin). Idempotent.
    async fn lift(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
