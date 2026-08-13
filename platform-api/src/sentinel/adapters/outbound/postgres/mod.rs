use platform_core::sentinel::domain::errors::DomainError;

// Wrappers Pg* pour les enums du domaine (sqlx::Type vit ici, pas dans core).
pub mod types;
pub mod uow;

// Bounded contexts (mirror de ports/outbound/).
pub mod ai;
pub mod audit;
pub mod community;
pub mod guild_backup;
pub mod moderation;
pub mod system;

/// Helper centralise : convertit une erreur sqlx en DomainError::Internal.
/// Utilise par tous les repositories Postgres.
pub(crate) fn pg_err(e: sqlx::Error) -> DomainError {
    DomainError::Internal(e.to_string())
}

/// Variante avec contexte (nom de table / repo). Le contexte apparait
/// dans le message d'erreur pour aider au debug : `"voice_channels pg: ..."`.
/// Remplace les ~14 fonctions `pg_err` locales redefinies dans chaque repo.
pub(crate) fn pg_err_ctx(ctx: &'static str, e: sqlx::Error) -> DomainError {
    DomainError::Internal(format!("{ctx} pg: {e}"))
}

/// Variante curryfiee de [`pg_err_ctx`] : capture le contexte et renvoie une
/// closure prete pour `.map_err(...)`. Evite la repetition de
/// `|e| pg_err_ctx("ctx", e)` sur ~120 sites de repositories.
pub(crate) fn pg_ctx(ctx: &'static str) -> impl FnOnce(sqlx::Error) -> DomainError {
    move |e| pg_err_ctx(ctx, e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_err_wraps_sqlx_row_not_found_into_internal() {
        let err = pg_err(sqlx::Error::RowNotFound);
        match err {
            DomainError::Internal(msg) => {
                assert!(!msg.is_empty());
            }
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn pg_err_wraps_protocol_error() {
        let err = pg_err(sqlx::Error::Protocol("connexion fermee".into()));
        match err {
            DomainError::Internal(msg) => assert!(msg.contains("connexion fermee")),
            other => panic!("expected Internal, got {other:?}"),
        }
    }

    #[test]
    fn pg_err_message_matches_display() {
        let source_err = sqlx::Error::PoolClosed;
        let display = source_err.to_string();
        let wrapped = pg_err(source_err);
        match wrapped {
            DomainError::Internal(msg) => assert_eq!(msg, display),
            other => panic!("expected Internal, got {other:?}"),
        }
    }
}
