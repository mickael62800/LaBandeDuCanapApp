//! Consolidation des jeux mentionnables : constat des divergences entre la
//! base et Discord, puis reparation dans la direction choisie par un humain.
//!
//! Aucune reparation automatique ici. Le rapport se lit, chaque ecart se
//! resout individuellement, et une suppression cote Discord passe toujours par
//! une demande explicite. Voir `platform-core` :
//! `domain::entities::casino::game_sync` (comparaison) et
//! `application::game_sync_service` (resolutions).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;
use platform_core::nexus::application::game_sync_service::GameSyncService;
use platform_core::nexus::domain::entities::casino::game_sync::{
    DiscordInventory, Divergence, SyncDirection,
};

fn service(state: &AppState) -> GameSyncService {
    GameSyncService::new(
        state.game_repo.clone(),
        state.game_sync_repo.clone(),
        state.events.clone(),
    )
}

// ── DTOs ──

#[derive(Debug, Serialize)]
pub struct DivergenceDto {
    /// Cle stable de la ligne, a renvoyer pour la resoudre.
    pub key: String,
    #[serde(flatten)]
    pub divergence: Divergence,
}

#[derive(Debug, Serialize)]
pub struct SyncReportDto {
    /// Date de la photographie ayant servi au calcul. `null` = le bot n'a
    /// jamais rendu compte : l'ecran doit dire « etat inconnu », surtout pas
    /// « tout va bien ».
    pub inventory_taken_at: Option<String>,
    pub divergences: Vec<DivergenceDto>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveDto {
    pub key: String,
    pub direction: SyncDirection,
}

#[derive(Debug, Serialize)]
pub struct ResolutionDto {
    pub key: String,
    /// La base a ete modifiee immediatement.
    pub applied_now: bool,
    /// Une demande a ete envoyee au bot ; son effet reste a confirmer par un
    /// prochain inventaire.
    pub requested_from_discord: bool,
    pub detail: String,
}

// ── Lecture ──

/// GET /api/games/{guild_id}/sync
pub async fn get_report(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<SyncReportDto>, ApiError> {
    let report = service(&state).report(&guild_id).await?;
    Ok(Json(SyncReportDto {
        inventory_taken_at: report.inventory_taken_at,
        divergences: report
            .divergences
            .into_iter()
            .map(|divergence| DivergenceDto {
                key: divergence.key(),
                divergence,
            })
            .collect(),
    }))
}

/// POST /api/games/{guild_id}/sync/check — demande un inventaire frais.
///
/// Repond 202 : la verification est LANCEE, pas terminee. Le bot repondra sur
/// `/sync/inventory` et le rapport changera ensuite.
pub async fn request_check(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    service(&state).request_inventory(&guild_id).await;
    Ok(StatusCode::ACCEPTED)
}

// ── Ecriture (bot) ──

/// PUT /api/games/{guild_id}/sync/inventory — le bot depose sa photographie.
pub async fn put_inventory(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(inventory): Json<DiscordInventory>,
) -> Result<StatusCode, ApiError> {
    service(&state)
        .record_inventory(&guild_id, &inventory)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct VanishedRoleDto {
    /// Jeux dont la liaison vient d'etre coupee.
    pub games: Vec<String>,
}

/// DELETE /api/games/{guild_id}/sync/roles/{role_id} — le bot a vu ce role
/// disparaitre de Discord.
///
/// La liaison est coupee tout de suite : la garder ferait echouer chaque
/// attribution jusqu'a la prochaine verification. Le jeu, lui, n'est jamais
/// supprime ici — cela reste une decision humaine.
pub async fn role_vanished(
    State(state): State<AppState>,
    Path((guild_id, role_id)): Path<(String, String)>,
) -> Result<Json<VanishedRoleDto>, ApiError> {
    let games = service(&state)
        .forget_vanished_role(&guild_id, &role_id)
        .await?;
    if !games.is_empty() {
        tracing::warn!(
            guild_id,
            role_id,
            jeux = ?games,
            "role de jeu disparu de Discord : liaison coupee"
        );
    }
    Ok(Json(VanishedRoleDto { games }))
}

// ── Resolution ──

/// POST /api/games/{guild_id}/sync/resolve
pub async fn resolve(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
    Json(dto): Json<ResolveDto>,
) -> Result<Json<ResolutionDto>, ApiError> {
    let outcome = service(&state)
        .resolve(&guild_id, &dto.key, dto.direction)
        .await?;
    Ok(Json(ResolutionDto {
        key: outcome.key,
        applied_now: outcome.applied_now,
        requested_from_discord: outcome.requested_from_discord,
        detail: outcome.detail,
    }))
}
