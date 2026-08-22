//! Port inbound : delai d'acceptation du reglement des arrivants ORDINAIRES.
//!
//! Le handler HTTP ne fait que parser et mapper ; le reglage de la guilde et le
//! calcul de l'echeance vivent dans le service, le SQL dans
//! `RulesDeadlineRepository`.
//!
//! Distinct de `ManageQuarantineUseCase`, qui traite les comptes SUSPECTS.

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::rules_deadline::RulesDeadlineSettings;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageRulesDeadlineUseCase: Send + Sync {
    /// Reglage de la guilde : delai, relance, expulsion.
    async fn settings(&self, guild_id: &str) -> Result<RulesDeadlineSettings, DomainError>;

    /// Ouvre le compte a rebours d'un arrivant.
    ///
    /// Sans effet si le delai n'est pas active, ou si une echeance existe deja
    /// pour ce membre : un evenement d'arrivee rejoue ne doit pas repousser
    /// l'echeance, ce qui offrirait un sursis illimite a qui sait le provoquer.
    ///
    /// Rend le reglage applique, pour que l'appelant annonce le vrai delai
    /// plutot qu'une valeur ecrite en dur.
    async fn start(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<RulesDeadlineSettings, DomainError>;

    /// Referme le compte a rebours : reglement accepte, ou membre parti.
    /// Idempotent.
    async fn clear(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
