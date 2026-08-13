//! Networks : liste (lecture seule).

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::ops::{ApiError, AppState};

#[derive(Debug, Serialize)]
pub struct NetworkDto {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub containers_count: usize,
}

pub async fn list_networks(
    State(state): State<AppState>,
) -> Result<Json<Vec<NetworkDto>>, ApiError> {
    let list = state.docker_host.list_networks().await?;
    let out: Vec<NetworkDto> = list
        .into_iter()
        .map(|n| NetworkDto {
            id: n.id,
            name: n.name,
            driver: n.driver,
            scope: n.scope,
            internal: n.internal,
            containers_count: n.containers_count,
        })
        .collect();
    Ok(Json(out))
}
