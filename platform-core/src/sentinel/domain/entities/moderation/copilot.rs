//! Entites du **Copilote de moderation** (feature lecture seule, consultative).
//!
//! Quand un moderateur demande du contexte sur un membre, le copilote assemble
//! son historique de moderation et une **suggestion de sanction proportionnee**
//! derivee (a) de l'echelle d'escalade des strikes du serveur et (b) de la
//! **jurisprudence** des decisions passees (comment les modos ont historiquement
//! tranche sur du contenu similaire). Le copilote n'APPLIQUE jamais rien : il
//! explique toujours le POURQUOI (`rationale` en francais).

use chrono::DateTime;
use chrono::Utc;

use crate::sentinel::domain::entities::moderation::review::automod::AppliedAction;

/// Distribution des precedents (jurisprudence) pour une categorie de flag
/// donnee : combien de fois chaque action a ete retenue sur des detections
/// similaires deja tranchees. Alimente le calcul de l'action modale.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrecedentDistribution {
    /// Categorie de flag dominante du membre (ex. `spam`, `insult`, `phishing`).
    /// Chaine vide si aucune categorie dominante n'a pu etre determinee.
    pub flag_category: String,
    /// Comptes par action (`action -> nombre`). Seules les reviews TRANCHEES
    /// (hors statut `voting`) sont comptees (anti-ancrage).
    pub counts_by_action: Vec<(String, u32)>,
    /// Nombre total de precedents comptes (= somme des `counts_by_action`).
    pub total: u32,
}

impl PrecedentDistribution {
    /// Distribution vide pour une categorie (aucun precedent).
    pub fn empty(flag_category: impl Into<String>) -> Self {
        Self {
            flag_category: flag_category.into(),
            counts_by_action: Vec::new(),
            total: 0,
        }
    }

    /// Action **modale** (la plus frequente) parmi les precedents. En cas
    /// d'egalite de comptes, on departage vers l'action la PLUS SEVERE
    /// (`AppliedAction::severity`), pour un resultat deterministe et prudent.
    /// `None` si aucun precedent ou aucune action reconnue.
    pub fn modal_action(&self) -> Option<AppliedAction> {
        self.counts_by_action
            .iter()
            .filter_map(|(action, count)| AppliedAction::from_str(action).map(|a| (a, *count)))
            .max_by(|(a_act, a_cnt), (b_act, b_cnt)| {
                a_cnt
                    .cmp(b_cnt)
                    .then_with(|| a_act.severity().cmp(&b_act.severity()))
            })
            .map(|(action, _)| action)
    }
}

/// Base ayant motive la suggestion — sert a l'explicabilite cote UI/bot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionBasis {
    /// Uniquement l'echelle d'escalade des strikes (precedents insuffisants).
    Escalation,
    /// Uniquement la jurisprudence (aucune escalade implicite disponible).
    Precedent,
    /// Escalade ET jurisprudence concordent/coexistent.
    Both,
    /// Aucune base exploitable : pas d'escalade et precedents insuffisants.
    Insufficient,
}

impl SuggestionBasis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Escalation => "escalation",
            Self::Precedent => "precedent",
            Self::Both => "both",
            Self::Insufficient => "insufficient",
        }
    }
}

/// Suggestion de sanction consultative (jamais appliquee). Toujours accompagnee
/// d'un `rationale` en francais expliquant le raisonnement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanctionSuggestion {
    /// Action suggeree. `None` = pas de recommandation (base insuffisante).
    pub action: Option<AppliedAction>,
    /// Fondement de la suggestion.
    pub basis: SuggestionBasis,
    /// Explication en francais (le POURQUOI). Toujours renseignee.
    pub rationale: String,
    /// Nombre de precedents pris en compte (= `PrecedentDistribution::total`).
    pub precedent_count: u32,
}

/// Contexte complet renvoye au moderateur pour un membre donne.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberModerationContext {
    /// Nombre de strikes actifs (dans la fenetre configuree).
    pub active_strikes: u32,
    /// Historique des sanctions appliquees par type (`type -> nombre`).
    pub sanctions_by_type: Vec<(String, u32)>,
    /// Date de la derniere sanction appliquee, le cas echeant.
    pub last_sanction_at: Option<DateTime<Utc>>,
    /// Nombre de reviews automod encore ouvertes visant ce membre.
    pub open_reviews: u32,
    /// Distribution des precedents pour la categorie dominante.
    pub precedents: PrecedentDistribution,
    /// Suggestion de sanction proportionnee + explication.
    pub suggestion: SanctionSuggestion,
}

#[cfg(test)]
#[path = "tests/copilot.rs"]
mod tests;
