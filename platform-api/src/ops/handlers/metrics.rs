//! Metriques Prometheus.

use axum::extract::State;
use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use crate::ops::AppConfig;

pub async fn metrics(State(config): State<Arc<AppConfig>>, headers: HeaderMap) -> Response {
    let supplied = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if !platform_common_api::metrics::metrics_auth_ok(Some(&config.metrics_token), supplied) {
        return (StatusCode::UNAUTHORIZED, "jeton metrics invalide").into_response();
    }
    platform_common_api::metrics::render_metrics()
}
