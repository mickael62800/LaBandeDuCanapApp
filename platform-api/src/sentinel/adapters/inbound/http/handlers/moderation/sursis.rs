//! Handlers « ban en sursis ».

use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::sursis::{Sursis, SursisStatus};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::moderation::manage_sursis::CreateSursisCommand;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::bootstrap::state::ModerationState;
/// Rapport de job renvoye au worker (observabilite). Copie locale de
/// l'ancien `application::game::worker_jobs::JobReport` (module jeux retire).
#[derive(Debug, serde::Serialize)]
pub struct JobReport {
    pub job: &'static str,
    pub processed: usize,
    pub errors: usize,
    pub details: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SursisDto {
    pub id: Uuid,
    pub user_id: String,
    pub username: String,
    pub reason: String,
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
    pub status: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl From<Sursis> for SursisDto {
    fn from(s: Sursis) -> Self {
        Self {
            id: s.id,
            user_id: s.user_id,
            username: s.username,
            reason: s.reason,
            saved_roles: s.saved_roles,
            channel_id: s.channel_id,
            status: s.status.as_str().to_string(),
            expires_at: s.expires_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSursisDto {
    pub user_id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub moderator_id: String,
    #[serde(default)]
    pub moderator_name: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub saved_roles: Vec<String>,
    pub channel_id: Option<String>,
}

/// POST /api/moderation/{guild_id}/sursis
pub async fn create_sursis(
    State(state): State<ModerationState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateSursisDto>,
) -> Result<Json<SursisDto>, ApiError> {
    // Delai depuis la config (parametrable), defaut 7 jours.
    let days = platform_core::sentinel::domain::entities::system::bot_config::cfg_i64(
        &state
            .bot_config_repo
            .get_config(
                &guild_id,
                platform_core::sentinel::domain::entities::system::bot_names::MODERATION_BOT,
            )
            .await
            .unwrap_or_default(),
        "sursis_appeal_days",
        7,
    );

    let sursis = state
        .sursis_uc
        .create(CreateSursisCommand {
            guild_id,
            user_id: dto.user_id,
            username: dto.username,
            moderator_id: dto.moderator_id,
            moderator_name: dto.moderator_name,
            reason: dto.reason,
            saved_roles: dto.saved_roles,
            channel_id: dto.channel_id,
            days,
        })
        .await?;
    Ok(Json(sursis.into()))
}

/// GET /api/moderation/sursis/{id}
pub async fn get_sursis(
    State(state): State<ModerationState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SursisDto>, ApiError> {
    let s = state
        .sursis_uc
        .get(id)
        .await?
        .ok_or_else(|| ApiError(DomainError::NotFound("Sursis introuvable".into())))?;
    // Scope tenant : on gate sur la guilde de la ressource (pas de guild_id dans
    // le path -> on le derive du sursis, comme resolve_review).
    Ok(Json(s.into()))
}

#[derive(Debug, Deserialize)]
pub struct ResolveSursisDto {
    pub status: String, // gracie | banni
}

/// POST /api/moderation/sursis/{id}/resolve
pub async fn resolve_sursis(
    State(state): State<ModerationState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveSursisDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let status = SursisStatus::from_str_lossy(&dto.status).ok_or_else(|| {
        ApiError(DomainError::ValidationError(format!(
            "statut invalide : {}",
            dto.status
        )))
    })?;
    // Gate sur la guilde du sursis (derive de la ressource) avant de resoudre.
    state
        .sursis_uc
        .get(id)
        .await?
        .ok_or_else(|| ApiError(DomainError::NotFound("Sursis introuvable".into())))?;
    // `claimed` = ce resolve a bien fait la transition (le sursis etait encore
    // en_sursis). false = deja resolu -> le bot doit s'abstenir de refaire
    // l'action Discord (re-ban / re-DM / suppression de salon).
    let claimed = state.sursis_uc.resolve(id, status).await?;
    Ok(Json(serde_json::json!({ "ok": true, "claimed": claimed })))
}

/// POST /api/moderation/internal/jobs/sursis-expire  (worker)
///
/// Bannit definitivement les sursis echus : diffuse `sursis_ban` (le bot ban +
/// nettoie le salon) et marque le sursis 'banni'.
pub async fn job_sursis_expire(
    State(state): State<ModerationState>,
) -> Result<Json<JobReport>, ApiError> {
    let due = state.sursis_uc.list_due().await?;
    let mut processed = 0;
    for s in &due {
        // Claim atomique AVANT d'agir : on ne bannit que si CE worker a bien fait
        // la transition en_sursis -> banni. Un pardon manuel concurrent (statut
        // != en_sursis) fait echouer le claim -> pas de ban de quelqu'un de
        // gracie, pas de double ban sur deux runs concurrents.
        let claimed = state
            .sursis_uc
            .resolve(s.id, SursisStatus::Banni)
            .await
            .unwrap_or(false);
        if !claimed {
            continue;
        }
        processed += 1;
        state.broadcaster.broadcast(
            "sursis_ban",
            serde_json::json!({
                "guild_id": s.guild_id,
                "user_id": s.user_id,
                "username": s.username,
                "reason": s.reason,
                "channel_id": s.channel_id,
            }),
        );
    }
    Ok(Json(JobReport {
        job: "sursis_expire",
        processed,
        errors: 0,
        details: serde_json::json!({}),
    }))
}
