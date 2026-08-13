//! Phase 6A — Handlers de la file d'attente des jobs d'export.
//!
//! Architecture identique a ai_jobs (Phase 4 A) : POST renvoie 202 immediat,
//! l'export-worker depile, execute la query, serialise le resultat et le
//! stocke inline dans `result` (TEXT). Les clients poll via GET pour recuperer.
//!
//! Controle d'acces : `auth_middleware` puis `superadmin_middleware`, poses au
//! niveau du routeur. Il n'y a plus de gate `Moderator+` propre a ces handlers
//! — le RBAC multi-roles a ete supprime (migration 007), le back-office n'a
//! qu'un utilisateur humain autorise.
//!
//! `GET` ne verifie pas non plus la propriete du job : connaitre l'UUID suffit.
//! Acceptable tant que tous les appelants sont l'administrateur unique et les
//! services internes ; a revoir le jour ou plusieurs comptes web coexistent,
//! puisqu'un export contient le dump de moderation d'un serveur entier.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::validation;
use crate::sentinel::bootstrap::state::SystemState;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::export_job_repository::{
    ExportJobRecord, NewExportJob,
};

#[derive(Debug, Deserialize)]
pub struct CreateExportJobDto {
    pub guild_id: GuildId,
    pub requested_by: String,
    /// "infractions" | "audit_logs" | "moderation_actions"
    pub job_type: String,
    /// "csv" | "json"
    pub format: String,
    #[serde(default)]
    pub filters: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ExportJobCreatedDto {
    pub job_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ExportJobStatusDto {
    pub id: Uuid,
    pub guild_id: String,
    pub requested_by: String,
    pub job_type: String,
    pub format: String,
    pub status: String,
    pub result: Option<String>,
    pub result_rows: Option<i32>,
    pub error_message: Option<String>,
    pub retries: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl From<ExportJobRecord> for ExportJobStatusDto {
    fn from(r: ExportJobRecord) -> Self {
        ExportJobStatusDto {
            id: r.id,
            guild_id: r.guild_id,
            requested_by: r.requested_by,
            job_type: r.job_type,
            format: r.format,
            status: r.status,
            result: r.result,
            result_rows: r.result_rows,
            error_message: r.error_message,
            retries: r.retries,
            created_at: r.created_at,
            started_at: r.started_at,
            completed_at: r.completed_at,
        }
    }
}

/// POST /api/exports/jobs — enqueue un job d'export. Retourne 202.
///
/// Le `guild_id` vient du CORPS, donc echappe au verrou mono-serveur (qui ne
/// lit que l'URL) — cf. `middleware/single_guild.rs` pour pourquoi c'est sans
/// consequence sur cette installation.
pub async fn create_export_job(
    State(state): State<SystemState>,
    Json(dto): Json<CreateExportJobDto>,
) -> Result<(StatusCode, Json<ExportJobCreatedDto>), ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("requested_by", &dto.requested_by).map_err(ApiError)?;

    if !platform_core::sentinel::domain::entities::system::job_whitelists::is_valid_export_job_type(
        &dto.job_type,
    ) {
        return Err(ApiError(DomainError::ValidationError(format!(
            "job_type invalide : '{}'",
            dto.job_type
        ))));
    }
    if !platform_core::sentinel::domain::entities::system::job_whitelists::is_valid_export_format(
        &dto.format,
    ) {
        return Err(ApiError(DomainError::ValidationError(format!(
            "format invalide : '{}' (attendu csv|json)",
            dto.format
        ))));
    }

    let id = state
        .export_jobs_uc
        .enqueue(NewExportJob {
            guild_id: dto.guild_id.as_str().to_string(),
            requested_by: dto.requested_by,
            job_type: dto.job_type,
            format: dto.format,
            filters: dto.filters,
        })
        .await?;

    Ok((
        StatusCode::ACCEPTED,
        Json(ExportJobCreatedDto {
            job_id: id.to_string(),
            status: "pending".into(),
        }),
    ))
}

/// GET /api/exports/jobs/{id} — statut + resultat (si done).
pub async fn get_export_job(
    State(state): State<SystemState>,
    Path(id): Path<String>,
) -> Result<Json<ExportJobStatusDto>, ApiError> {
    let uuid = validation::parse_uuid("job_id", &id).map_err(ApiError)?;

    let job = state
        .export_jobs_uc
        .get(uuid)
        .await?
        .ok_or_else(|| ApiError(DomainError::NotFound(format!("export_job {id}"))))?;

    Ok(Json(ExportJobStatusDto::from(job)))
}

#[cfg(test)]
#[path = "tests/exports.rs"]
mod tests;
