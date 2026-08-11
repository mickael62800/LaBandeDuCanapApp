//! Images : liste et suppression.

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::audit::{actor_from, audited};
use crate::handlers::ok_response;
use crate::{ApiError, AppState};

#[derive(Debug, Serialize)]
pub struct ImageDto {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: i64,
    pub size_bytes: i64,
    pub shared_size_bytes: i64,
    pub virtual_size_bytes: i64,
    pub containers: i64,
    pub dangling: bool,
}

pub async fn list_images(State(state): State<AppState>) -> Result<Json<Vec<ImageDto>>, ApiError> {
    let list = state.docker_host.list_images().await?;
    let out: Vec<ImageDto> = list
        .into_iter()
        .map(|i| {
            let dangling =
                i.repo_tags.is_empty() || i.repo_tags.iter().all(|t| t == "<none>:<none>");
            ImageDto {
                id: i.id,
                repo_tags: i.repo_tags,
                repo_digests: i.repo_digests,
                created: i.created,
                size_bytes: i.size,
                shared_size_bytes: i.shared_size,
                virtual_size_bytes: i.virtual_size,
                containers: i.containers,
                dangling,
            }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Debug, Deserialize)]
pub struct RemoveImageQuery {
    #[serde(default)]
    pub force: Option<bool>,
    #[serde(default)]
    pub no_prune: Option<bool>,
}

pub async fn remove_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<RemoveImageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = actor_from(&headers);
    audited(
        &state,
        &actor,
        "image.remove",
        &id,
        state
            .docker_host
            .remove_image(&id, q.force.unwrap_or(false), q.no_prune.unwrap_or(false)),
    )
    .await?;
    Ok(ok_response())
}
