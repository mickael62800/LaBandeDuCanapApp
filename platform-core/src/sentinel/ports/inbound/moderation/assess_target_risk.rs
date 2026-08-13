//! Port inbound : evaluation server-side du risque d'une cible de moderation.
//!
//! Le bot appelle ce use case (jamais la regle en dur) avant une action
//! destructive : il fournit les FAITS Discord de la cible, le use case applique
//! le SEUIL serveur + la POLITIQUE et renvoie la decision `risky + raison`.

use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::target_risk::TargetRiskDecision;
use crate::sentinel::domain::errors::DomainError;

/// Commande d'evaluation : faits Discord collectes par le bot pour `guild_id`.
#[derive(Debug, Clone)]
pub struct AssessTargetRiskCommand {
    pub guild_id: String,
    pub account_age_days: i64,
    pub is_bot: bool,
    pub has_mod_perms: bool,
}

#[async_trait]
pub trait AssessTargetRiskUseCase: Send + Sync {
    /// Decide si la cible est a risque (exige une confirmation) en appliquant le
    /// seuil serveur (`recent_account_days`) + la politique metier aux faits.
    async fn assess(&self, cmd: AssessTargetRiskCommand)
        -> Result<TargetRiskDecision, DomainError>;
}
