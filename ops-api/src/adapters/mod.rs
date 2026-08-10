//! Adaptateurs sortants (Postgres).

pub mod alert_rule_repository;

use ops_core::domain::errors::DomainError;

/// Traduction unique des erreurs sqlx. Le detail technique reste dans les
/// logs : le remonter au client ne renseignerait que sur le schema.
pub fn pg_err(error: sqlx::Error) -> DomainError {
    match error {
        sqlx::Error::RowNotFound => DomainError::NotFound("ressource introuvable".into()),
        other => {
            tracing::error!(%other, "erreur Postgres");
            DomainError::Internal("acces base impossible".into())
        }
    }
}