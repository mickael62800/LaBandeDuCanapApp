//! Helpers pour convertir les erreurs techniques (sqlx, redis, etc.) en
//! `DomainError` contextualisees. Evite la duplication de `map_err` inline
//! du type `|e| ApiError(DomainError::Internal(format!("contexte: {e}")))`.
//!
//! Usage :
//! ```ignore
//! use crate::sentinel::adapters::inbound::http::errors_helpers::sqlx_internal;
//! sqlx::query!("...").execute(pool).await.map_err(sqlx_internal("fetch voice channel"))?;
//! ```
//!
//! Comme `ApiError` implemente `From<DomainError>`, le `?` convertit
//! automatiquement — pas besoin de `.map_err(ApiError)` supplementaire.

use platform_core::sentinel::domain::errors::DomainError;

/// Wrap une erreur sqlx dans `DomainError::Internal` avec un prefixe de contexte.
/// Retourne une closure consommable une fois (FnOnce).
pub fn sqlx_internal(ctx: &str) -> impl FnOnce(sqlx::Error) -> DomainError + '_ {
    move |e| DomainError::Internal(format!("{ctx}: {e}"))
}

/// Wrap une erreur generique `Display` dans `DomainError::Internal` avec contexte.
pub fn internal_with<E: std::fmt::Display>(ctx: &str) -> impl FnOnce(E) -> DomainError + '_ {
    move |e| DomainError::Internal(format!("{ctx}: {e}"))
}

/// Wrap une erreur generique `Display` dans `DomainError::ValidationError` avec contexte.
pub fn validation_with<E: std::fmt::Display>(ctx: &str) -> impl FnOnce(E) -> DomainError + '_ {
    move |e| DomainError::ValidationError(format!("{ctx}: {e}"))
}

#[cfg(test)]
#[path = "tests/errors_helpers.rs"]
mod tests;
