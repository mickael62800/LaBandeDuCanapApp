//! GET /containers/changes — instantane courant + derniers changements.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use crate::{ApiError, AppState};

#[derive(serde::Serialize)]
pub struct ContainerChangesResponse {
    pub last_check: String,
    pub current: Vec<ops_core::domain::entities::container_monitor::ContainerSnapshot>,
    pub changes_24h: Vec<ops_core::domain::entities::container_monitor::ContainerChangeEntry>,
}

/// Lit l'etat publie dans Redis par `ops-worker`.
pub async fn container_changes(
    State(state): State<AppState>,
) -> Result<Json<ContainerChangesResponse>, ApiError> {
    let snapshot = crate::container_monitor::load(&state.redis_client)
        .await
        .map_err(|error| {
            tracing::warn!(%error, "lecture du snapshot conteneurs impossible");
            ApiError(StatusCode::SERVICE_UNAVAILABLE, "snapshot indisponible".into())
        })?;
    Ok(Json(ContainerChangesResponse {
        last_check: snapshot.last_check,
        current: snapshot.current,
        changes_24h: snapshot.recent_changes,
    }))
}
