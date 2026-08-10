//! Adaptateurs sortants (Postgres).

pub mod alert_rule_repository;
pub mod security_log_repository;
pub mod security_audit_repository;
pub mod ip_ban_repository;
pub mod host_security;
pub mod geoip;
pub mod http_docker_host;
pub mod server_event_repository;
pub mod redis_log_stream;
pub mod log_repository;

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
/// Variante contextualisee : le ctx identifie la requete dans les logs.
///
/// Le message technique ne quitte pas les logs — ce que voit l'appelant reste
/// generique, une trace SQL dans le navigateur ne servant qu'a un attaquant.
pub fn pg_err_ctx(ctx: &'static str, error: sqlx::Error) -> DomainError {
    tracing::error!(%error, ctx, "erreur Postgres");
    DomainError::Internal("acces base impossible".into())
}

/// Forme curryfiee, prete pour .map_err(...).
pub fn pg_ctx(ctx: &'static str) -> impl FnOnce(sqlx::Error) -> DomainError {
    move |error| pg_err_ctx(ctx, error)
}