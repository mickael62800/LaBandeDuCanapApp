//! Prune : purges par ressource + prune systeme (containers/images/networks
//! + volumes optionnels) et build cache.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::audit::{actor_from, audited, record_docker_audit};
use crate::{ApiError, AppState};

#[derive(Debug, Serialize)]
pub struct PruneResultDto {
    pub deleted: Vec<String>,
    pub space_reclaimed_bytes: u64,
}

fn prune_dto(o: ops_core::domain::entities::docker_host::PruneOutcome) -> PruneResultDto {
    PruneResultDto {
        deleted: o.deleted,
        space_reclaimed_bytes: o.space_reclaimed_bytes,
    }
}

/// Resultat d'UNE etape d'un prune systeme, succes ou echec.
///
/// Le prune systeme est destructif et sequentiel : si une etape ulterieure
/// echoue, les precedentes ont deja ete appliquees. On expose donc le resultat
/// de chaque etape plutot que d'abandonner sur la premiere erreur, sinon
/// l'operateur ne saurait pas ce qui a reellement ete supprime.
#[derive(Debug, Serialize)]
pub struct PruneStepDto {
    pub ok: bool,
    pub deleted: Vec<String>,
    pub space_reclaimed_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn prune_step(
    result: Result<
        ops_core::domain::entities::docker_host::PruneOutcome,
        ops_core::domain::errors::DomainError,
    >,
) -> PruneStepDto {
    match result {
        Ok(o) => PruneStepDto {
            ok: true,
            deleted: o.deleted,
            space_reclaimed_bytes: o.space_reclaimed_bytes,
            error: None,
        },
        Err(error) => PruneStepDto {
            ok: false,
            deleted: Vec::new(),
            space_reclaimed_bytes: 0,
            error: Some(error.to_string()),
        },
    }
}

/// Etape non demandee (ex. volumes sans `?volumes=true`) : succes neutre.
fn prune_step_skipped() -> PruneStepDto {
    PruneStepDto {
        ok: true,
        deleted: Vec::new(),
        space_reclaimed_bytes: 0,
        error: None,
    }
}

pub async fn prune_containers(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PruneResultDto>, ApiError> {
    let actor = actor_from(&headers);
    let r = audited(
        &state,
        &actor,
        "prune.containers",
        "*",
        state.docker_host.prune_containers(),
    )
    .await?;
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
    let actor = actor_from(&headers);
    let target = if q.all.unwrap_or(false) {
        "all=true"
    } else {
        "dangling=true"
    };
    let r = audited(
        &state,
        &actor,
        "prune.images",
        target,
        state.docker_host.prune_images(q.all.unwrap_or(false)),
    )
    .await?;
    Ok(Json(prune_dto(r)))
}

pub async fn prune_volumes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PruneResultDto>, ApiError> {
    let actor = actor_from(&headers);
    let r = audited(
        &state,
        &actor,
        "prune.volumes",
        "*",
        state.docker_host.prune_volumes(),
    )
    .await?;
    Ok(Json(prune_dto(r)))
}

pub async fn prune_networks(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PruneResultDto>, ApiError> {
    let actor = actor_from(&headers);
    let r = audited(
        &state,
        &actor,
        "prune.networks",
        "*",
        state.docker_host.prune_networks(),
    )
    .await?;
    Ok(Json(prune_dto(r)))
}

#[derive(Debug, Serialize)]
pub struct PruneSystemDto {
    pub containers: PruneStepDto,
    pub images: PruneStepDto,
    pub volumes: PruneStepDto,
    pub networks: PruneStepDto,
    pub total_space_reclaimed_bytes: u64,
    /// `false` si au moins une etape a echoue (echec partiel).
    pub all_succeeded: bool,
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
    let actor = actor_from(&headers);
    let target = format!(
        "volumes={},all_images={}",
        q.volumes.unwrap_or(false),
        q.all_images.unwrap_or(false)
    );

    // Etapes sequentielles (dependances fonctionnelles), mais on capture le
    // resultat de CHACUNE : un echec en cours de route n'annule pas ce qui a
    // deja ete supprime, et l'operateur doit le voir.
    let containers = prune_step(state.docker_host.prune_containers().await);
    let images = prune_step(
        state
            .docker_host
            .prune_images(q.all_images.unwrap_or(false))
            .await,
    );
    let networks = prune_step(state.docker_host.prune_networks().await);
    let volumes = if q.volumes.unwrap_or(false) {
        prune_step(state.docker_host.prune_volumes().await)
    } else {
        prune_step_skipped()
    };

    // `networks` etait oublie du total : l'espace recupere etait sous-estime.
    let total = containers.space_reclaimed_bytes
        + images.space_reclaimed_bytes
        + volumes.space_reclaimed_bytes
        + networks.space_reclaimed_bytes;
    let all_succeeded = containers.ok && images.ok && volumes.ok && networks.ok;

    // Audit APRES les etapes, avec l'issue reelle et les etapes en echec.
    let failed: Vec<&str> = [
        ("containers", containers.ok),
        ("images", images.ok),
        ("volumes", volumes.ok),
        ("networks", networks.ok),
    ]
    .iter()
    .filter(|(_, ok)| !ok)
    .map(|(name, _)| *name)
    .collect();
    let audit_error = (!failed.is_empty()).then(|| failed.join(", "));
    record_docker_audit(
        &state,
        &actor,
        "prune.system",
        &target,
        all_succeeded,
        audit_error.as_deref(),
    )
    .await;

    Ok(Json(PruneSystemDto {
        total_space_reclaimed_bytes: total,
        all_succeeded,
        containers,
        images,
        volumes,
        networks,
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
    let all = q.all.unwrap_or(true);
    let actor = actor_from(&headers);
    let target = if all { "all=true" } else { "all=false" };
    let r = audited(
        &state,
        &actor,
        "prune.build_cache",
        target,
        state.docker_host.prune_build_cache(all),
    )
    .await?;
    Ok(Json(prune_dto(r)))
}
