//! Adaptateurs sortants PARTAGES par `ops-api` et `ops-worker`.
//!
//! # Pourquoi cette crate existe
//!
//! `ops-worker` ne reutilisait que deux adaptateurs — le client du `docker-agent`
//! (`HttpDockerHost`) et le repo Postgres des events serveur
//! (`PgServerEventRepository`) — mais dependait pour ca de la crate `ops-api`
//! ENTIERE : il compilait tout le transport Axum, les handlers et l'etat de
//! l'API sans jamais les appeler, et une modif d'un handler pouvait casser son
//! build. Ces deux adaptateurs vivent desormais ici ; `ops-api` et `ops-worker`
//! en dependent a egalite, et le Worker ne dépend plus de l'API.

pub mod http_docker_host;
pub mod server_event_repository;

use platform_core::ops::domain::errors::DomainError;

/// Traduction des erreurs sqlx en `DomainError`. Le detail technique reste dans
/// les logs : le remonter a l'appelant ne renseignerait que sur le schema.
///
/// Duplique volontairement le helper equivalent d'`ops-api` : il fait six
/// lignes, et les deux crates n'ont pas a se dependre pour lui.
pub(crate) fn pg_err(error: sqlx::Error) -> DomainError {
    match error {
        sqlx::Error::RowNotFound => DomainError::NotFound("ressource introuvable".into()),
        other => {
            tracing::error!(%other, "erreur Postgres");
            DomainError::Internal("acces base impossible".into())
        }
    }
}
