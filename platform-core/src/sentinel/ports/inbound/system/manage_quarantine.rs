//! Port inbound : gestion des quarantaines de securite. Le handler HTTP ne fait
//! que parser/RBAC/mapper ; la regle metier (delai avant kick) vit dans le
//! service, le SQL dans `QuarantineRepository`.

use async_trait::async_trait;

use crate::sentinel::domain::entities::system::quarantine::{ActiveQuarantine, QuarantineSettings};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageQuarantineUseCase: Send + Sync {
    /// Reglage de la guilde pour les membres en attente d'acceptation du
    /// reglement (delai, rappel, expulsion). Sert aussi au bot pour ecrire un
    /// message qui annonce le vrai delai.
    async fn settings(&self, guild_id: &str) -> Result<QuarantineSettings, DomainError>;

    /// Met (ou remet) un membre en quarantaine ; une re-quarantaine reset le
    /// timer (idempotent).
    ///
    /// `timeout_secs` vaut normalement `None` : le delai vient du reglage de la
    /// guilde, seule source de verite. `Some` reste possible pour un appel
    /// deliberement explicite (outil d'administration).
    ///
    /// Retourne le reglage effectivement applique, pour que l'appelant annonce
    /// au membre le delai reel plutot qu'une valeur ecrite en dur.
    async fn quarantine_user(
        &self,
        guild_id: &str,
        user_id: &str,
        timeout_secs: Option<i64>,
    ) -> Result<QuarantineSettings, DomainError>;

    /// Liste les quarantaines encore actives (non expirees).
    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError>;

    /// Leve la quarantaine d'un membre (captcha valide ou retrait admin). Idempotent.
    async fn lift(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
