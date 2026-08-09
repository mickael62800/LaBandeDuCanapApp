use super::*;
use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use sentinel_core::domain::errors::DomainError;

async fn response_parts(err: ApiError) -> (StatusCode, serde_json::Value) {
    let resp = err.into_response();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    (status, json)
}

#[tokio::test]
async fn not_found_maps_to_404() {
    let (status, body) = response_parts(ApiError(DomainError::NotFound("user".into()))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body["error"].as_str().unwrap().contains("user"));
}

#[tokio::test]
async fn rule_not_found_maps_to_404() {
    let (status, _) = response_parts(ApiError(DomainError::NotFound(format!(
        "Regle {}",
        uuid::Uuid::nil()
    ))))
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn infraction_not_found_maps_to_404() {
    let (status, _) = response_parts(ApiError(DomainError::NotFound(format!(
        "Infraction {}",
        uuid::Uuid::nil()
    ))))
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn ticket_not_found_maps_to_404() {
    let (status, _) = response_parts(ApiError(DomainError::NotFound("Ticket t".to_string()))).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn invalid_rule_maps_to_422() {
    // Une regle invalide est une erreur de validation -> 422 (la variante
    // dediee historique a ete fusionnee dans DomainError::ValidationError).
    let (status, _) = response_parts(ApiError(DomainError::ValidationError("x".into()))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn validation_error_maps_to_422() {
    let (status, body) =
        response_parts(ApiError(DomainError::ValidationError("champ vide".into()))).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("vide"));
}

#[tokio::test]
async fn forbidden_maps_to_403() {
    let (status, _) = response_parts(ApiError(DomainError::Forbidden("nope".into()))).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn conflict_maps_to_409() {
    let (status, _) = response_parts(ApiError(DomainError::Conflict("dup".into()))).await;
    assert_eq!(status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn rate_limited_maps_to_429() {
    let (status, _) = response_parts(ApiError(DomainError::RateLimited("slow".into()))).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn timeout_maps_to_504() {
    let (status, _) = response_parts(ApiError(DomainError::Timeout("slow".into()))).await;
    assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
}

#[tokio::test]
async fn internal_maps_to_500_and_hides_detail() {
    let (status, body) =
        response_parts(ApiError(DomainError::Internal("SECRET SQL err".into()))).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let msg = body["error"].as_str().unwrap();
    assert!(!msg.contains("SECRET"));
    assert_eq!(msg, "Erreur interne");
}

#[test]
fn api_error_from_domain_error() {
    let err: ApiError = DomainError::NotFound("x".into()).into();
    assert!(matches!(err.0, DomainError::NotFound(_)));
}
