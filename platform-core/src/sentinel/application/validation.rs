//! Helpers de validation partages par les services applicatifs.
//!
//! Evite de repeter le garde `if x.trim().is_empty() { return Err(...) }`
//! dans chaque methode de service.

use crate::sentinel::domain::errors::DomainError;

/// Plafond standard d'une page de listing (endpoints web).
pub const PAGE_LIMIT_MAX: i64 = 500;
/// Plafond des listings « batch » (exports, agrégations, dataset).
pub const BATCH_LIMIT_MAX: i64 = 1000;

/// Valide qu'un `guild_id` n'est pas vide. Message coherent partout.
pub fn validate_guild_id(guild_id: &str) -> Result<(), DomainError> {
    validate_non_empty(guild_id, "guild_id")
}

/// Valide qu'un champ texte n'est pas vide (apres trim).
pub fn validate_non_empty(value: &str, field: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::ValidationError(format!("{field} requis")))
    } else {
        Ok(())
    }
}

/// Valide qu'un montant est strictement positif.
pub fn validate_positive(amount: i64, label: &str) -> Result<(), DomainError> {
    if amount <= 0 {
        Err(DomainError::ValidationError(format!(
            "{label} doit etre positif"
        )))
    } else {
        Ok(())
    }
}

/// Valide qu'une valeur est dans l'intervalle inclusif `[lo, hi]`.
pub fn validate_range(value: i64, lo: i64, hi: i64, field: &str) -> Result<(), DomainError> {
    if value < lo || value > hi {
        Err(DomainError::ValidationError(format!(
            "{field} doit etre entre {lo} et {hi}"
        )))
    } else {
        Ok(())
    }
}
