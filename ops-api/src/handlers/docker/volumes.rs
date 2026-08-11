//! Volumes : liste et suppression.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::audit::{actor_from, audited};
use crate::handlers::ok_response;
use crate::{ApiError, AppState};

#[derive(Debug, Serialize)]
pub struct VolumeDto {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
    pub size_bytes: Option<i64>,
    pub ref_count: Option<i64>,
    pub in_use: bool,
}

pub async fn list_volumes(State(state): State<AppState>) -> Result<Json<Vec<VolumeDto>>, ApiError> {
    let list = state.docker_host.list_volumes().await?;
    let out: Vec<VolumeDto> = list
        .into_iter()
        .map(|v| VolumeDto {
            name: v.name,
            driver: v.driver,
            mountpoint: v.mountpoint,
            created_at: v.created_at,
            size_bytes: v.size,
            ref_count: v.ref_count,
            in_use: v.ref_count.map(|r| r > 0).unwrap_or(false),
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct RemoveVolumeQuery {
    #[serde(default)]
    pub force: Option<bool>,
}

pub async fn remove_volume(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(q): Query<RemoveVolumeQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = actor_from(&headers);
    audited(
        &state,
        &actor,
        "volume.remove",
        &name,
        state.docker_host.remove_volume(&name, q.force.unwrap_or(false)),
    )
    .await?;
    Ok(ok_response())
}
