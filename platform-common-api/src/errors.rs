use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Json;
use platform_common::errors::DomainError;

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
        (status, Json(serde_json::json!({ "error": msg }))).into_response()
    }
}
