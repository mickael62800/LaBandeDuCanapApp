//! Service de domaine PUR du copilote de moderation : `suggest_sanction`.
//!
//! Aucune I/O. Deterministe et explicable. Combine deux signaux pour proposer
//! une sanction proportionnee (consultative, jamais appliquee) :
//!   1. **Escalade** : ce que l'echelle des strikes du serveur implique pour la
//!      PROCHAINE infraction (`active_strikes + 1`), via la logique de ladder
//!      partagee `escalation_for`.
//!   2. **Jurisprudence** : l'action MODALE des precedents tranches sur la
//!      categorie de flag dominante du membre, si et seulement si le nombre de
//!      precedents atteint `min_precedents`.
//!
//! Combinaison :
//!   - Precedents suffisants -> on suit la jurisprudence (`Both` si une escalade
//!     existe aussi, sinon `Precedent`).
//!   - Precedents insuffisants mais escalade disponible -> `Escalation`.
//!   - Ni l'un ni l'autre -> `Insufficient`, aucune suggestion.

use crate::sentinel::domain::entities::moderation::action::strikes::escalation_for;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeThreshold;
use crate::sentinel::domain::entities::moderation::copilot::PrecedentDistribution;
use crate::sentinel::domain::entities::moderation::copilot::SanctionSuggestion;
use crate::sentinel::domain::entities::moderation::copilot::SuggestionBasis;
use crate::sentinel::domain::entities::moderation::review::automod::AppliedAction;

/// Entrees de la suggestion (faits deja rassembles par la couche application).
pub struct SuggestInputs<'a> {
    /// Strikes actifs du membre (fenetre configuree).
    pub active_strikes: u32,
    /// Seuils d'escalade configures pour le serveur (ladder, ordre libre).
    pub thresholds: &'a [StrikeThreshold],
    /// Distribution des precedents pour la categorie dominante.
    pub precedents: &'a PrecedentDistribution,
    /// Nombre minimal de precedents requis pour suivre la jurisprudence.
    pub min_precedents: u32,
}

/// Calcule la suggestion de sanction (fonction pure).
pub fn suggest_sanction(input: &SuggestInputs) -> SanctionSuggestion {
    let precedent_count = input.precedents.total;
    let min = input.min_precedents.max(1);

    // 1. Action impliquee par l'escalade pour la PROCHAINE infraction.
    let next_offense = input.active_strikes.saturating_add(1);
    let escalation = escalation_for(input.thresholds, next_offense)
        .and_then(|(action, _)| AppliedAction::from_str(&action));

    // 2. Action modale de la jurisprudence, seulement si assez de precedents.
    let precedents_sufficient = precedent_count >= min;
    let modal = if precedents_sufficient {
        input.precedents.modal_action()
    } else {
        None
    };

    // 3. Combinaison.
    match (modal, escalation.clone()) {
        // Jurisprudence exploitable : on la suit, en citant l'escalade si elle existe.
        (Some(modal_action), esc) => {
            let basis = if esc.is_some() {
                SuggestionBasis::Both
            } else {
                SuggestionBasis::Precedent
            };
            let mut rationale = format!(
                "Jurisprudence : sur {} precedent(s) « {} » tranche(s), l'action la plus frequente est « {} » ({}).",
                precedent_count,
                display_category(&input.precedents.flag_category),
                modal_action.as_str(),
                describe_counts(input.precedents),
            );
            match esc {
                Some(esc_action) => {
                    rationale.push_str(&format!(
                        " L'echelle des strikes du serveur implique « {} » pour la prochaine infraction ({} strike(s) actif(s)).",
                        esc_action.as_str(),
                        input.active_strikes,
                    ));
                }
                None => {
                    rationale.push_str(" Aucune escalade de strikes applicable a ce stade.");
                }
            }
            SanctionSuggestion {
                action: Some(modal_action),
                basis,
                rationale,
                precedent_count,
            }
        }
        // Pas de jurisprudence exploitable mais une escalade existe.
        (None, Some(esc_action)) => {
            let why = if precedent_count == 0 {
                "aucun precedent disponible".to_string()
            } else {
                format!("pas assez de precedents (n={precedent_count} < min={min})")
            };
            let rationale = format!(
                "Suggestion basee sur l'echelle des strikes : « {} » pour la prochaine infraction ({} strike(s) actif(s)). Jurisprudence non retenue : {}.",
                esc_action.as_str(),
                input.active_strikes,
                why,
            );
            SanctionSuggestion {
                action: Some(esc_action),
                basis: SuggestionBasis::Escalation,
                rationale,
                precedent_count,
            }
        }
        // Ni escalade ni jurisprudence.
        (None, None) => {
            let why = if precedent_count == 0 {
                "aucun precedent disponible".to_string()
            } else {
                format!("pas assez de precedents (n={precedent_count} < min={min})")
            };
            let rationale = format!(
                "Aucune suggestion : ni escalade de strikes applicable ({} strike(s) actif(s)), ni jurisprudence exploitable ({}). Decision laissee a l'appreciation du moderateur.",
                input.active_strikes, why,
            );
            SanctionSuggestion {
                action: None,
                basis: SuggestionBasis::Insufficient,
                rationale,
                precedent_count,
            }
        }
    }
}

fn display_category(category: &str) -> &str {
    if category.trim().is_empty() {
        "(non categorise)"
    } else {
        category
    }
}

/// Rend lisible la repartition des precedents, ex. `ban x3, mute x1`.
fn describe_counts(precedents: &PrecedentDistribution) -> String {
    if precedents.counts_by_action.is_empty() {
        return "aucun detail".to_string();
    }
    precedents
        .counts_by_action
        .iter()
        .map(|(action, count)| format!("{action} x{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
#[path = "tests/moderation_copilot.rs"]
mod tests;
