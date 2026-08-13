//! Port inbound : evaluation d'une declaration d'age au reglement.
//!
//! Le handler HTTP appelle ce use case (jamais le repo config directement) :
//! la DECISION (seuil pass/ban + duree du ban) est server-side. Le bot ne fait
//! qu'appliquer l'action Discord retournee.

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::age_check::AgeCheckDecision;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait EvaluateAgeDeclarationUseCase: Send + Sync {
    /// Decide l'issue d'une declaration d'age pour `{guild_id, user_id}` :
    /// lit la config serveur (age minimum + duree de ban par annee) et applique
    /// la regle metier.
    async fn evaluate(
        &self,
        guild_id: &str,
        user_id: &str,
        declared_age: i32,
    ) -> Result<AgeCheckDecision, DomainError>;
}
