use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use platform_common::errors::DomainError;

/// Construit l'enveloppe d'erreur JSON commune a toutes les APIs.
pub fn error_response(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

/// Masque les details techniques des erreurs serveur et les journalise.
pub fn public_message(status: StatusCode, error: &impl std::fmt::Display) -> String {
    if status.is_server_error() {
        tracing::error!(%error, "erreur interne");
        "erreur interne".to_owned()
    } else {
        error.to_string()
    }
}

/// Enveloppe d'erreur API : mappe DomainError -> statut HTTP + JSON.
pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        Self(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self.0 {
            DomainError::Validation(m) | DomainError::ValidationError(m) => {
                (StatusCode::UNPROCESSABLE_ENTITY, m.clone()) // UNPROCESSABLE_ENTITY (422) is used in sentinel, BAD_REQUEST (400) was in nexus. Let's use 422 for both.
            }
            DomainError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
            DomainError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            DomainError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
            DomainError::RateLimited(m) => (StatusCode::TOO_MANY_REQUESTS, m.clone()),
            DomainError::Timeout(m) => (StatusCode::GATEWAY_TIMEOUT, m.clone()),
            DomainError::NotImplemented(m) => (StatusCode::NOT_IMPLEMENTED, m.clone()),
            DomainError::Infrastructure(m) | DomainError::Internal(m) => {
                tracing::error!(error = %m, "erreur interne");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Erreur interne".to_string(), // sentinel uses "Erreur interne", nexus "erreur interne".
                )
            }
        };
        error_response(status, &msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masque_uniquement_les_erreurs_serveur() {
        assert_eq!(
            public_message(StatusCode::INTERNAL_SERVER_ERROR, &"sql password=secret"),
            "erreur interne"
        );
        assert_eq!(
            public_message(StatusCode::BAD_REQUEST, &"champ invalide"),
            "champ invalide"
        );
    }
}
