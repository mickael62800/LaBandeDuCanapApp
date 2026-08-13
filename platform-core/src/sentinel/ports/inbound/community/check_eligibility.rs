//! Use case inbound Community : DECISIONS d'eligibilite (server-side).
//!
//! Le bot fournit uniquement les donnees Discord (roles actuels, dates de join
//! Discord — zone grise legitime : seul le bot y a acces) ; la lecture de la
//! config et l'evaluation des regles/seuils vivent ici (domaine + config port).

use async_trait::async_trait;

use crate::sentinel::domain::entities::community::eligibility::EligibilityDecision;
use crate::sentinel::domain::errors::DomainError;

/// Verifie l'eligibilite d'un membre a un role (prerequis `role_prerequisites`).
#[derive(Debug, Clone)]
pub struct CheckRoleEligibilityCommand {
    pub guild_id: String,
    pub role_id: u64,
    /// Roles Discord actuels du membre (fournis par le bot).
    pub user_roles: Vec<u64>,
    /// Timestamp unix (s) de join Discord. `None` => 0 jour d'anciennete
    /// (reproduit le `unwrap_or(0)` historique du bot).
    pub joined_at_unix: Option<i64>,
}

/// Valide un parrainage (anti-self + seuils d'anciennete de config).
#[derive(Debug, Clone)]
pub struct ValidateSponsorshipCommand {
    pub guild_id: String,
    pub sponsor_id: u64,
    pub sponsored_id: u64,
    /// Join Discord du parrain. `None` => 0 jour (echoue le min, comme le bot).
    pub sponsor_joined_at_unix: Option<i64>,
    /// Join Discord du filleul. `None` => u64::MAX jours (echoue le max, idem bot).
    pub sponsored_joined_at_unix: Option<i64>,
}

#[async_trait]
pub trait CheckEligibilityUseCase: Send + Sync {
    /// Decide si le membre remplit les prerequis du role.
    async fn check_role_eligibility(
        &self,
        cmd: CheckRoleEligibilityCommand,
    ) -> Result<EligibilityDecision, DomainError>;

    /// Decide si le parrainage respecte les regles de config.
    async fn validate_sponsorship(
        &self,
        cmd: ValidateSponsorshipCommand,
    ) -> Result<EligibilityDecision, DomainError>;
}
