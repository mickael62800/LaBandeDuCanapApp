use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    /// 404 — ressource introuvable. Le message est libre.
    #[error("Ressource introuvable : {0}")]
    NotFound(String),

    /// 400 / 422 — donnees invalides.
    #[error("Donnees invalides : {0}")]
    ValidationError(String),

    /// Alias pour ValidationError (utilisé dans nexus-core).
    #[error("{0}")]
    Validation(String),

    /// 409 — conflit (unique constraint, version stale, etc.).
    #[error("Conflit : {0}")]
    Conflict(String),

    /// 403 — acces refuse.
    #[error("Acces refuse : {0}")]
    Forbidden(String),

    /// 429 — rate limit depasse.
    #[error("Rate limited : {0}")]
    RateLimited(String),

    /// 504 — timeout sur appel externe.
    #[error("Timeout : {0}")]
    Timeout(String),

    /// 500 — erreur interne (sqlx, redis, runtime). Message technique pour debug.
    #[error("Erreur interne : {0}")]
    Internal(String),

    /// Erreur d'infrastructure (DB, reseau) remontee par un adapter (utilisé dans nexus-core).
    #[error("erreur infrastructure: {0}")]
    Infrastructure(String),

    /// 501 — methode non implementee.
    #[error("Non implemente : {0}")]
    NotImplemented(String),
}
