//! GET /api/docker/overview — synthese df + version du daemon.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use crate::{ApiError, AppState};
use ops_core::domain::entities::docker_host::compute_overview;

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

pub async fn get_overview(State(state): State<AppState>) -> Result<Json<OverviewDto>, ApiError> {
    // Deux appels independants au Docker Agent : en parallele, la latence de
    // l'overview tend vers celle du plus lent au lieu de leur somme.
    let (info, usage) = tokio::try_join!(
        state.docker_host.version_info(),
        state.docker_host.disk_usage()
    )?;
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
