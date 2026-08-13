//! DTO de sortie du copilote de moderation (serde, lecture seule).
//! Miroir du contexte de domaine `MemberModerationContext`.

use serde::Serialize;

use platform_core::sentinel::domain::entities::moderation::copilot::MemberModerationContext;
use platform_core::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use platform_core::sentinel::domain::entities::moderation::copilot::SanctionSuggestion;

/// Compte d'une action (ex. sanction par type, precedent par action).
#[derive(Debug, Serialize)]
pub struct ActionCountDto {
    /// Type d'action (`warn`, `mute`, `ban`, ...).
    pub action: String,
    /// Nombre d'occurrences.
    pub count: u32,
}

/// Distribution des precedents (jurisprudence) pour la categorie dominante.
#[derive(Debug, Serialize)]
pub struct PrecedentDistributionDto {
    /// Categorie de flag dominante (`spam`, `insult`, ...). Vide si inconnue.
    pub flag_category: String,
    /// Comptes par action retenue (hors reviews `voting`).
    pub counts_by_action: Vec<ActionCountDto>,
    /// Nombre total de precedents comptes.
    pub total: u32,
}

/// Suggestion de sanction consultative + explication (POURQUOI).
#[derive(Debug, Serialize)]
pub struct SanctionSuggestionDto {
    /// Action suggeree (`warn`|`mute`|`ban`|...) ou `null` si base insuffisante.
    pub action: Option<String>,
    /// Fondement : `escalation` | `precedent` | `both` | `insufficient`.
    pub basis: String,
    /// Explication en francais du raisonnement.
    pub rationale: String,
    /// Nombre de precedents pris en compte.
    pub precedent_count: u32,
}

/// Contexte complet renvoye au moderateur.
#[derive(Debug, Serialize)]
pub struct MemberModerationContextDto {
    /// Strikes actifs (fenetre configuree).
    pub active_strikes: u32,
    /// Historique de sanctions appliquees par type.
    pub sanctions_by_type: Vec<ActionCountDto>,
    /// Derniere sanction appliquee (RFC 3339), le cas echeant.
    pub last_sanction_at: Option<String>,
    /// Reviews automod encore ouvertes visant le membre.
    pub open_reviews: u32,
    /// Distribution des precedents (jurisprudence).
    pub precedents: PrecedentDistributionDto,
    /// Suggestion de sanction proportionnee.
    pub suggestion: SanctionSuggestionDto,
}

fn counts_to_dto(counts: Vec<(String, u32)>) -> Vec<ActionCountDto> {
    counts
        .into_iter()
        .map(|(action, count)| ActionCountDto { action, count })
        .collect()
}

impl From<PrecedentDistribution> for PrecedentDistributionDto {
    fn from(p: PrecedentDistribution) -> Self {
        Self {
            flag_category: p.flag_category,
            counts_by_action: counts_to_dto(p.counts_by_action),
            total: p.total,
        }
    }
}

impl From<SanctionSuggestion> for SanctionSuggestionDto {
    fn from(s: SanctionSuggestion) -> Self {
        Self {
            action: s.action.map(|a| a.as_str().to_string()),
            basis: s.basis.as_str().to_string(),
            rationale: s.rationale,
            precedent_count: s.precedent_count,
        }
    }
}

impl From<MemberModerationContext> for MemberModerationContextDto {
    fn from(c: MemberModerationContext) -> Self {
        Self {
            active_strikes: c.active_strikes,
            sanctions_by_type: counts_to_dto(c.sanctions_by_type),
            last_sanction_at: c.last_sanction_at.map(|d| d.to_rfc3339()),
            open_reviews: c.open_reviews,
            precedents: c.precedents.into(),
            suggestion: c.suggestion.into(),
        }
    }
}

#[cfg(test)]
#[path = "tests/copilot.rs"]
mod tests;
