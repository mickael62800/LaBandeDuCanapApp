//! Garde commune aux endpoints publics.
//!
//! Ces routes sont montees HORS de la pile d'authentification : n'importe qui
//! sur Internet peut les appeler avec n'importe quel parametre. La validation
//! stricte du `guild_id` est donc le premier filtre, pas une politesse — elle
//! evite qu'une chaine arbitraire atteigne la couche de persistance.

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use platform_core::sentinel::domain::errors::DomainError;

/// Un identifiant Discord est un entier 64 bits en decimal : au plus 20
/// chiffres, rien d'autre.
pub fn ensure_guild_id(guild_id: &str) -> Result<(), ApiError> {
    if guild_id.is_empty() || guild_id.len() > 20 || !guild_id.chars().all(|c| c.is_ascii_digit()) {
        return Err(ApiError(DomainError::ValidationError(
            "guild_id invalide".into(),
        )));
    }
    Ok(())
}

/// Borne une limite venue du client.
///
/// Sans borne, `?limit=1000000` transformerait une page publique en moyen de
/// saturer la base depuis l'exterieur.
pub fn clamp_limit(requested: Option<i64>, default: i64, max: i64) -> i64 {
    requested.unwrap_or(default).clamp(1, max)
}

#[cfg(test)]
#[path = "tests/public_guard.rs"]
mod tests;
