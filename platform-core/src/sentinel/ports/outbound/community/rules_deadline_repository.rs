//! Port outbound : echeances d'acceptation du reglement des arrivants
//! (`welcome_rules_pending`). Tout le SQL vit dans l'adapter Postgres.
//!
//! Distinct de `QuarantineRepository`, qui suit les comptes SUSPECTS : les deux
//! files n'ont ni la meme population ni la meme issue, et les melanger ferait
//! qu'un reglage de securite deplace l'echeance d'un membre legitime.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::sentinel::domain::entities::community::rules_deadline::PendingRulesDeadline;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait RulesDeadlineRepository: Send + Sync {
    /// Pose l'echeance d'un arrivant, SANS ecraser celle qui existe deja.
    ///
    /// Un membre qui repasse par l'accueil — reconnexion, evenement Discord
    /// rejoue, redemarrage du bot — ne doit pas voir son compte a rebours
    /// reparti de zero : ce serait un sursis illimite pour qui sait provoquer
    /// l'evenement.
    async fn insert_if_absent(
        &self,
        guild_id: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError>;

    /// Echeances dont la relance n'est pas encore partie et dont la fenetre est
    /// ouverte. `reminded_at` est pose par l'appelant AVANT publication.
    async fn list_reminder_due(&self, limit: i64)
        -> Result<Vec<PendingRulesDeadline>, DomainError>;

    /// Marque la relance comme envoyee. Rend `false` si elle l'etait deja :
    /// c'est la garde qui empeche deux instances de relancer le meme membre.
    async fn claim_reminder(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError>;

    /// Echeances echues.
    async fn list_expired(&self, limit: i64) -> Result<Vec<PendingRulesDeadline>, DomainError>;

    /// Retire l'echeance (idempotent). Appele a l'acceptation du reglement, au
    /// depart du membre, et apres une expulsion.
    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
