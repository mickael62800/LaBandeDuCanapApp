//! GET/POST/DELETE /api/docker/* — administration Docker via le port `DockerHost`.
//!
//! Toutes les actions destructives (start/stop/restart/delete/prune) sont gardees
//! par require_superadmin. Les GET listing/inspect sont gates par moderator+ via
//! le middleware standard (suffisant : ils n'exposent que des metadonnees techniques).
//!
//! Le client Docker (bollard) vit dans l'adapter outbound
//! `adapters::outbound::system::docker_host::BollardDockerHost` ; l'agregation
//! « reclaimable » est une fonction pure du core (`compute_overview`).
//! Necessite que /var/run/docker.sock soit monte (RW) dans le conteneur API.

use std::collections::HashMap;

use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::ApiError;

use crate::AppState;
use ops_core::domain::entities::docker_host::compute_overview;

use crate::{authorize, handlers::ok_response};

/// Helper d'audit log pour les actions Docker destructives.
/// Tracking via tracing::info! structure -> apparait dans les logs API
/// avec actor.user_id, action, target. Permet de retrouver qui a lance
/// quoi en cas de probleme post-mortem.
/// Logue en `tracing::info` ET en BDD `server_events` pour qu'il soit visible
/// sur la page Securite serveur.
fn audit_docker(state: &AppState, actor: &str, action: &str, target: &str) {
    tracing::info!(
        target: "audit::docker",
        actor = %actor,
        action = action,
        target = target,
        "action d'administration Docker"
    );
    let repo = state.server_events.clone();
    let actor = actor.to_owned();
    let action_qualifiee = format!("docker.{action}");
    let cible = target.to_owned();
    // Purges et suppressions sont irreversibles : niveau plus visible que la
    // simple consultation.
    let severite = if action.contains("prune") || action.contains("remove") {
        "warn"
    } else {
        "info"
    };
    // Detache : journaliser ne doit ni retarder la reponse a l'operateur, ni
    // faire echouer une action Docker qui, elle, a bien eu lieu.
    tokio::spawn(async move {
        if let Err(error) = repo
            .record(
                &actor,
                None,
                &action_qualifiee,
                Some(&cible),
                severite,
                serde_json::json!({}),
            )
            .await
        {
            tracing::warn!(%error, "journalisation de l'action Docker impossible");
        }
    });
}

/// Identifiant Discord de l'operateur, remonte par nginx depuis uth_request.
///
/// ops-api ne sait pas resoudre une session Discord. Sans cette remontee,
/// l'audit des actions destructives perdrait son auteur : on saurait qu'un
/// conteneur a ete supprime, jamais par qui.
fn actor_from(headers: &HeaderMap) -> String {
    headers
        .get("x-actor-id")
        .and_then(|v| v.to_str().ok())
        .filter(|v| !v.is_empty())
        .unwrap_or("inconnu")
        .to_owned()
}
// ── DTOs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct OverviewDto {
    pub version: String,
    pub api_version: String,
    pub os: String,
    pub arch: String,
    pub kernel: String,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images_count: i64,
    pub volumes_count: i64,
    pub networks_count: i64,
    pub layers_size_bytes: i64,
    pub images_size_bytes: i64,
    pub containers_size_bytes: i64,
    pub volumes_size_bytes: i64,
    pub build_cache_size_bytes: i64,
    pub reclaimable_images_bytes: i64,
    pub reclaimable_containers_bytes: i64,
    pub reclaimable_volumes_bytes: i64,
    pub reclaimable_build_cache_bytes: i64,
}

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

#[derive(Debug, Serialize)]
pub struct NetworkDto {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub containers_count: usize,
}

#[derive(Debug, Serialize)]
pub struct PruneResultDto {
    pub deleted: Vec<String>,
    pub space_reclaimed_bytes: u64,
}

// ── Overview (df + version) ───────────────────────────────────────────────

pub async fn get_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OverviewDto>, ApiError> {
    authorize(&headers, &state.config)?;
    let info = state.docker_host.version_info().await?;
    let usage = state.docker_host.disk_usage().await?;
    let agg = compute_overview(&usage);

    Ok(Json(OverviewDto {
        version: info.version,
        api_version: info.api_version,
        os: info.os,
        arch: info.arch,
        kernel: info.kernel,
        containers_running: info.containers_running,
        containers_paused: info.containers_paused,
        containers_stopped: info.containers_stopped,
        images_count: info.images_count,
        volumes_count: agg.volumes_count,
        networks_count: 0, // rempli par list_networks separement si besoin
        layers_size_bytes: agg.layers_size_bytes,
        images_size_bytes: agg.images_size_bytes,
        containers_size_bytes: agg.containers_size_bytes,
        volumes_size_bytes: agg.volumes_size_bytes,
        build_cache_size_bytes: agg.build_cache_size_bytes,
        reclaimable_images_bytes: agg.reclaimable_images_bytes,
        reclaimable_containers_bytes: agg.reclaimable_containers_bytes,
        reclaimable_volumes_bytes: agg.reclaimable_volumes_bytes,
        reclaimable_build_cache_bytes: agg.reclaimable_build_cache_bytes,
    }))
}

// ── Containers ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ListContainersQuery {
    #[serde(default)]
    pub all: Option<bool>,
}

pub async fn list_containers(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListContainersQuery>,
) -> Result<Json<Vec<ContainerDto>>, ApiError> {
    authorize(&headers, &state.config)?;
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
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "container.start", &id);
    state.docker_host.start_container(&id).await?;
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
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "container.stop", &id);
    state
        .docker_host
        .stop_container(&id, q.timeout.unwrap_or(10))
        .await?;
    Ok(ok_response())
}

pub async fn restart_container(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<StopQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "container.restart", &id);
    state
        .docker_host
        .restart_container(&id, q.timeout.unwrap_or(10))
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
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "container.remove", &id);
    state
        .docker_host
        .remove_container(&id, q.force.unwrap_or(false), q.volumes.unwrap_or(false))
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

pub async fn container_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<LogsDto>, ApiError> {
    authorize(&headers, &state.config)?;
    let tail = q.tail.unwrap_or(200).min(5000);
    let logs = state
        .docker_host
        .container_logs(&id, tail, q.timestamps.unwrap_or(false))
        .await?;
    Ok(Json(LogsDto { logs }))
}

// ── Images ────────────────────────────────────────────────────────────────

pub async fn list_images(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<ImageDto>>, ApiError> {
    authorize(&headers, &state.config)?;
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
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "image.remove", &id);
    state
        .docker_host
        .remove_image(&id, q.force.unwrap_or(false), q.no_prune.unwrap_or(false))
        .await?;
    Ok(ok_response())
}

// ── Volumes ───────────────────────────────────────────────────────────────

pub async fn list_volumes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<VolumeDto>>, ApiError> {
    authorize(&headers, &state.config)?;
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

pub async fn remove_volume(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Query(q): Query<RemoveImageQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "volume.remove", &name);
    state
        .docker_host
        .remove_volume(&name, q.force.unwrap_or(false))
        .await?;
    Ok(ok_response())
}

// ── Networks ──────────────────────────────────────────────────────────────

pub async fn list_networks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<NetworkDto>>, ApiError> {
    authorize(&headers, &state.config)?;
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

// ── Prune ─────────────────────────────────────────────────────────────────

fn prune_dto(o: ops_core::domain::entities::docker_host::PruneOutcome) -> PruneResultDto {
    PruneResultDto {
        deleted: o.deleted,
        space_reclaimed_bytes: o.space_reclaimed_bytes,
    }
}

pub async fn prune_containers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PruneResultDto>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "prune.containers", "*");
    let r = state.docker_host.prune_containers().await?;
    Ok(Json(prune_dto(r)))
}

#[derive(Debug, Deserialize)]
pub struct PruneImagesQuery {
    /// Si `true` : supprime aussi les images non taggees mais utilisees nulle part.
    /// Si `false` (defaut) : seulement les "dangling" (sans tag).
    #[serde(default)]
    pub all: Option<bool>,
}

pub async fn prune_images(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PruneImagesQuery>,
) -> Result<Json<PruneResultDto>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(
        &state,
        &actor_from(&headers),
        "prune.images",
        if q.all.unwrap_or(false) {
            "all=true"
        } else {
            "dangling=true"
        },
    );
    let r = state
        .docker_host
        .prune_images(q.all.unwrap_or(false))
        .await?;
    Ok(Json(prune_dto(r)))
}

pub async fn prune_volumes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PruneResultDto>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "prune.volumes", "*");
    let r = state.docker_host.prune_volumes().await?;
    Ok(Json(prune_dto(r)))
}

pub async fn prune_networks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PruneResultDto>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(&state, &actor_from(&headers), "prune.networks", "*");
    let r = state.docker_host.prune_networks().await?;
    Ok(Json(prune_dto(r)))
}

#[derive(Debug, Serialize)]
pub struct PruneSystemDto {
    pub containers: PruneResultDto,
    pub images: PruneResultDto,
    pub volumes: PruneResultDto,
    pub networks: PruneResultDto,
    pub total_space_reclaimed_bytes: u64,
}

#[derive(Debug, Deserialize)]
pub struct PruneSystemQuery {
    #[serde(default)]
    pub volumes: Option<bool>,
    #[serde(default)]
    pub all_images: Option<bool>,
}

/// POST /api/docker/prune/system — prune complet (containers + images + networks
/// + volumes optionnels). Equivalent `docker system prune` cote CLI.
pub async fn prune_system(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PruneSystemQuery>,
) -> Result<Json<PruneSystemDto>, ApiError> {
    authorize(&headers, &state.config)?;
    audit_docker(
        &state,
        &actor_from(&headers),
        "prune.system",
        &format!(
            "volumes={},all_images={}",
            q.volumes.unwrap_or(false),
            q.all_images.unwrap_or(false)
        ),
    );

    let containers = state.docker_host.prune_containers().await?;
    let images = state
        .docker_host
        .prune_images(q.all_images.unwrap_or(false))
        .await?;
    let networks = state.docker_host.prune_networks().await?;
    let volumes = if q.volumes.unwrap_or(false) {
        state.docker_host.prune_volumes().await?
    } else {
        ops_core::domain::entities::docker_host::PruneOutcome::default()
    };

    let total = containers.space_reclaimed_bytes
        + images.space_reclaimed_bytes
        + volumes.space_reclaimed_bytes;

    Ok(Json(PruneSystemDto {
        total_space_reclaimed_bytes: total,
        containers: prune_dto(containers),
        images: prune_dto(images),
        volumes: prune_dto(volumes),
        networks: prune_dto(networks),
    }))
}

#[derive(Debug, Deserialize)]
pub struct PruneBuildCacheQuery {
    /// Si `true` (defaut) : purge TOUT le build cache (`docker builder prune -a`).
    /// Si `false` : seulement les entrees inutilisees/non ancrees.
    #[serde(default)]
    pub all: Option<bool>,
}

/// POST /api/docker/prune/build-cache — purge le build cache Docker (buildkit).
pub async fn prune_build_cache(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<PruneBuildCacheQuery>,
) -> Result<Json<PruneResultDto>, ApiError> {
    authorize(&headers, &state.config)?;
    let all = q.all.unwrap_or(true);
    audit_docker(
        &state,
        &actor_from(&headers),
        "prune.build_cache",
        if all { "all=true" } else { "all=false" },
    );
    let r = state.docker_host.prune_build_cache(all).await?;
    Ok(Json(prune_dto(r)))
}

// ── Instantane de surveillance ────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct ContainerChangesResponse {
    pub last_check: String,
    pub current: Vec<ops_core::domain::entities::container_monitor::ContainerSnapshot>,
    pub changes_24h: Vec<ops_core::domain::entities::container_monitor::ContainerChangeEntry>,
}

/// GET /containers/changes — instantane courant + derniers changements.
///
/// Lit l'etat partage alimente par container_monitor, dans ce meme
/// processus : aucune requete Docker ici, la reponse est immediate.
pub async fn container_changes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ContainerChangesResponse>, ApiError> {
    authorize(&headers, &state.config)?;
    let snapshot = state.container_monitor.read().await;
    Ok(Json(ContainerChangesResponse {
        last_check: snapshot.last_check.clone(),
        current: snapshot.current.clone(),
        changes_24h: snapshot.recent_changes.clone(),
    }))
}
