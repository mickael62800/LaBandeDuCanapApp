//! Phase 4 A — Handlers de la file d'attente des jobs IA.
//!
//! Approche queue async : les bots POSTent un job (retour 202 immediat avec
//! `job_id`) au lieu d'attendre la reponse synchrone des endpoints `/analyze`.
//! L'ai-worker depile et appelle les services d'inference. Les bots peuvent
//! soit poll `GET /api/ai/jobs/:id`, soit ecouter Redis `ai_result:{job_id}`.
//!
//! Adaptateur ENTRANT mince : parse/map uniquement. La validation
//! (job_type whitelist, guild_id) vit dans `ManageAiJobsUseCase` ; le SQL
//! dans `AiJobRepository`.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::AiState;
use platform_core::sentinel::domain::entities::ai::ai_job::NewAiJob;

#[derive(Debug, Deserialize)]
pub struct CreateAiJobDto {
    pub guild_id: String,
    /// "analyze_text" ou "analyze_image"
    pub job_type: String,
    pub input_payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AiJobCreatedDto {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct AiJobStatusDto {
    pub id: Uuid,
    pub guild_id: String,
    pub job_type: String,
    pub status: String,
    pub input_payload: serde_json::Value,
    pub result_payload: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub retries: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// POST /api/ai/jobs — enqueue un job IA. Retourne 202 immediatement.
pub async fn create_ai_job(
    State(state): State<AiState>,
    Json(dto): Json<CreateAiJobDto>,
) -> Result<(StatusCode, Json<AiJobCreatedDto>), ApiError> {
    let id = state
        .ai_jobs_uc
        .create_job(NewAiJob {
            guild_id: dto.guild_id,
            job_type: dto.job_type,
            input_payload: dto.input_payload,
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(AiJobCreatedDto {
            job_id: id.to_string(),
            status: "pending".to_string(),
        }),
    ))
}

/// GET /api/ai/jobs/{id} — recupere le statut courant d'un job IA.
pub async fn get_ai_job(
    State(state): State<AiState>,
    Path(id): Path<String>,
) -> Result<Json<AiJobStatusDto>, ApiError> {
    let uuid = validation::parse_uuid("job_id", &id).map_err(ApiError)?;

    let job = state.ai_jobs_uc.get_job(uuid).await?;

    Ok(Json(AiJobStatusDto {
        id: job.id,
        guild_id: job.guild_id,
        job_type: job.job_type,
        status: job.status,
        input_payload: job.input_payload,
        result_payload: job.result_payload,
        error_message: job.error_message,
        retries: job.retries,
        created_at: job.created_at,
        started_at: job.started_at,
        completed_at: job.completed_at,
    }))
}

#[cfg(test)]
#[path = "tests/ai_jobs.rs"]
mod tests;
