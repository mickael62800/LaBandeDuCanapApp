pub mod bot_config;
pub mod casino;
pub mod coussin;
pub mod game;
pub mod grand_salon;
pub mod wallet;
pub mod wheel;

pub use crate::shared::errors::ApiError;
use platform_core::nexus::domain::errors::DomainError;

/// Validation minimale d'un snowflake Discord (remplace le module
/// `validation` de sentinel-api) : chiffres uniquement, longueur bornee.
pub fn validate_discord_id(field: &str, value: &str) -> Result<(), DomainError> {
    let ok = !value.is_empty()
        && value.len() <= 20
        && value.len() >= 15
        && value.chars().all(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(DomainError::ValidationError(format!(
            "{field} invalide : snowflake Discord attendu"
        )))
    }
}
