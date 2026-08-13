//! DTO de l'evaluation server-side du risque d'une cible de moderation.
//!
//! Entree : les FAITS Discord collectes par le bot. Sortie : la DECISION
//! (`risky` + `reason`) appliquee par le use case.

use serde::{Deserialize, Serialize};

use platform_core::sentinel::domain::entities::moderation::target_risk::TargetRiskDecision;

/// Faits Discord de la cible envoyes par le bot.
#[derive(Debug, Deserialize)]
pub struct AssessTargetRiskRequestDto {
    /// Age du compte Discord en jours (>= 0).
    pub account_age_days: i64,
    /// La cible est un bot.
    pub is_bot: bool,
    /// La cible possede des permissions de moderation (staff).
    pub has_mod_perms: bool,
}

/// Decision renvoyee au bot : arme (ou non) la modale de confirmation.
#[derive(Debug, Serialize)]
pub struct TargetRiskDecisionDto {
    pub risky: bool,
    /// Motif du risque (affiche au moderateur) ou `null` si non risque.
    pub reason: Option<String>,
}

impl From<TargetRiskDecision> for TargetRiskDecisionDto {
    fn from(d: TargetRiskDecision) -> Self {
        Self {
            risky: d.risky,
            reason: d.reason,
        }
    }
}
