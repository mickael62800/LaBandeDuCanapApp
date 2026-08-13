//! Conteneurs : liste, cycle de vie (start/stop/restart/remove) et logs.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::audit::{actor_from, audited};
use crate::ops::handlers::ok_response;
use crate::ops::{ApiError, AppState};

#[derive(Debug, Serialize)]
pub struct ContainerDto {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: i64,
    pub size_rw_bytes: Option<i64>,
    pub size_root_fs_bytes: Option<i64>,
    pub ports: Vec<String>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct ListContainersQuery {
    #[serde(default)]
    pub all: Option<bool>,
}

pub async fn list_containers(
    State(state): State<AppState>,
    Query(q): Query<ListContainersQuery>,
) -> Result<Json<Vec<ContainerDto>>, ApiError> {
    let list = state
        .docker_host
        .list_containers(q.all.unwrap_or(true))
        .await?;
    let out: Vec<ContainerDto> = list
        .into_iter()
        .map(|c| ContainerDto {
            id: c.id,
            names: c.names,
            image: c.image,
            state: c.state,
            status: c.status,
            created: c.created,
            size_rw_bytes: c.size_rw,
            size_root_fs_bytes: c.size_root_fs,
            ports: c
                .ports
                .into_iter()
                .map(|p| match p.public_port {
                    Some(pub_port) => format!("{}:{}/{}", pub_port, p.private_port, p.protocol),
                    None => format!("{}/{}", p.private_port, p.protocol),
                })
                .collect(),
            labels: c.labels,
        })
        .collect();
    Ok(Json(out))
}

pub async fn start_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = actor_from(&headers);
    audited(
        &state,
        &actor,
        "container.start",
        &id,
        state.docker_host.start_container(&id),
    )
    .await?;
    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct StopQuery {
    #[serde(default)]
    pub timeout: Option<i64>,
}

pub async fn stop_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = actor_from(&headers);
    audited(
        &state,
        &actor,
        "container.stop",
        &id,
        state
            .docker_host
            .stop_container(&id, q.timeout.unwrap_or(10)),
    )
    .await?;
    Ok(ok_response())
}

pub async fn restart_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = actor_from(&headers);
    audited(
        &state,
        &actor,
        "container.restart",
        &id,
        state
            .docker_host
            .restart_container(&id, q.timeout.unwrap_or(10)),
    )
    .await?;
    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct RemoveContainerQuery {
    #[serde(default)]
    pub force: Option<bool>,
    #[serde(default)]
    pub volumes: Option<bool>,
}

pub async fn remove_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<RemoveContainerQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = actor_from(&headers);
    audited(
        &state,
        &actor,
        "container.remove",
        &id,
        state.docker_host.remove_container(
            &id,
            q.force.unwrap_or(false),
            q.volumes.unwrap_or(false),
        ),
    )
    .await?;
    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub tail: Option<u32>,
    #[serde(default)]
    pub timestamps: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct LogsDto {
    pub logs: String,
}

/// Lecture des logs d'un conteneur — AUDITEE au meme titre que les actions
/// destructives.
///
/// Elle ne modifie rien, mais c'est l'operation la plus exposante de cette
/// surface : les logs d'`auth-api`, de `postgres` ou d'`api` contiennent
/// couramment des jetons, des chaines de connexion en cas d'erreur et des
/// donnees d'utilisateurs. Tracer les suppressions sans tracer les lectures
/// aurait laisse l'exfiltration comme seule action invisible du journal.
pub async fn container_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<LogsDto>, ApiError> {
    let actor = actor_from(&headers);
    let tail = q.tail.unwrap_or(200).min(5000);
    let logs = audited(
        &state,
        &actor,
        "container.logs",
        &id,
        state
            .docker_host
            .container_logs(&id, tail, q.timestamps.unwrap_or(false)),
    )
    .await?;
    Ok(Json(LogsDto { logs }))
}
