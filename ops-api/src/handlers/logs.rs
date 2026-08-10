use axum::extract::{Path, Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::adapters::redis_log_stream;
use crate::AppState;
use crate::ApiError;
use ops_core::domain::entities::log_entry::LogEntry;

#[derive(Deserialize)]
pub struct GuildFilterParams {
    pub category: Option<String>,
    pub level: Option<String>,
    pub guild_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    200
}

#[derive(Serialize)]
pub struct LogEntryDto {
    pub id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub bot: String,
    pub server: String,
    pub message: String,
    pub category: String,
    pub details: serde_json::Value,
}

impl From<LogEntry> for LogEntryDto {
    fn from(entry: LogEntry) -> Self {
        Self {
            id: entry.id,
            timestamp: entry.timestamp,
            level: entry.level,
            bot: entry.bot,
            server: entry.server,
            message: entry.message,
            category: entry.category,
            details: entry.details,
        }
    }
}

fn normalize_in(value: i64, default: i64, min: i64, max: i64) -> i64 {
    if value < min || value > max {
        default
    } else {
        value
    }
}

/// GET /ops-api/logs — logs récents (filtrable par guild_id, category, level).
pub async fn get_logs(
    State(state): State<AppState>,
    Query(params): Query<GuildFilterParams>,
) -> Result<Json<Vec<LogEntryDto>>, ApiError> {
    let limit = normalize_in(params.limit, 200, 1, 1000);

    let dtos: Vec<LogEntryDto> = if let Some(cat) = params.category.as_deref() {
        // Cache Redis (adapter) : filtre guild post-fetch faute d'index.
        redis_log_stream::xrevrange_logs(
            &state.redis_client,
            cat,
            params.level.as_deref(),
            limit as usize,
        )
        .await
        .into_iter()
        .filter(|l| params.guild_id.as_ref().is_none_or(|gid| l.server == *gid))
        .map(LogEntryDto::from)
        .collect()
    } else {
        // Postgres : le use case pousse le filtre guild dans la requete.
        let filters = ops_core::ports::inbound::manage_system_logs::SystemLogFilters {
            category: None,
            level: params.level.clone(),
            guild_id: params.guild_id.clone(),
            limit,
        };
        state
            .system_logs_uc
            .list_logs(filters)
            .await
            .map_err(|e| ApiError::from_domain(&e))?
            .into_iter()
            .map(LogEntryDto::from)
            .collect()
    };

    Ok(Json(dtos))
}

/// DELETE /ops-api/logs/{category} — supprimer tous les logs d'une categorie
pub async fn delete_logs_by_category(
    State(state): State<AppState>,
    Path(category): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .system_logs_uc
        .purge_category(&category)
        .await
        .map_err(|e| ApiError::from_domain(&e))?;
    redis_log_stream::delete_stream(&state.redis_client, &category).await;
    Ok(Json(serde_json::json!({ "deleted": count })))
}
