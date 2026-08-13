//! Handler `/metrics` de nexus-api.
//!
//! Toute la mecanique (recorder, middleware de comptage, sampler tokio) vit
//! dans `platform-common-api::metrics`, partagee avec `sentinel-api`. Seul le
//! handler reste ici : il lit le jeton dans l'etat applicatif, donc il ne peut
//! pas etre generique sans abstraire cet etat pour rien.

use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use platform_common_api::metrics::{metrics_auth_ok, render_metrics};

use crate::nexus::bootstrap::AppState;

pub use platform_common_api::metrics::init_prometheus;
pub use platform_common_api::metrics::metrics_middleware;
pub use platform_common_api::metrics::spawn_tokio_runtime_sampler;

/// GET /metrics — format texte Prometheus.
///
/// Protege par `NEXUS_METRICS_TOKEN` si la variable est definie. Vide = ouvert,
/// ce qui convient au reseau Docker interne ou seul Prometheus scrape.
pub async fn metrics_handler(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    if !metrics_auth_ok(state.metrics_token.as_deref(), auth_header) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    render_metrics()
}
