//! Service d'export : delegate les queries au port `ExportRepository`,
//! serialise en CSV ou JSON. Pure logic — no infra.

use std::sync::Arc;

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::system::export_repository::{
    AuditLogExport, ExportRepository, InfractionExport, ModerationActionExport,
};

/// Resultat d'un export : donnees serialisees + nombre de lignes.
#[derive(Debug)]
pub struct ExportResult {
    pub data: String,
    pub row_count: usize,
}

#[async_trait]
pub trait ExecuteExportUseCase: Send + Sync {
    async fn execute(
        &self,
        guild_id: &str,
        job_type: &str,
        format: &str,
        max_rows: i64,
    ) -> Result<ExportResult, DomainError>;
}

pub struct ExportService {
    repo: Arc<dyn ExportRepository>,
}

impl ExportService {
    pub fn new(repo: Arc<dyn ExportRepository>) -> Self {
        Self { repo }
    }
}

/// Bornes saines du cap de lignes par export — source unique, partagée avec
/// le clamp de config de sentinel-worker (`max_rows_per_export`).
pub const EXPORT_MAX_ROWS_MIN: i64 = 1;
pub const EXPORT_MAX_ROWS_MAX: i64 = 50_000;

/// Clamp du cap de lignes par export dans les bornes saines.
pub fn clamp_export_rows(v: i64) -> i64 {
    v.clamp(EXPORT_MAX_ROWS_MIN, EXPORT_MAX_ROWS_MAX)
}

#[async_trait]
impl ExecuteExportUseCase for ExportService {
    async fn execute(
        &self,
        guild_id: &str,
        job_type: &str,
        format: &str,
        max_rows: i64,
    ) -> Result<ExportResult, DomainError> {
        let max_rows = clamp_export_rows(max_rows);
        match job_type {
            "infractions" => {
                let rows = self.repo.fetch_infractions(guild_id, max_rows).await?;
                serialize_rows(
                    &rows,
                    format,
                    |r: &InfractionExport| {
                        vec![
                            r.id.to_string(),
                            r.channel_id.clone(),
                            r.user_id.clone(),
                            r.username.clone(),
                            r.message_id.clone(),
                            r.content.clone(),
                            format!("{:.3}", r.score),
                            r.action.clone(),
                            r.reason.clone(),
                            r.duration.map(|d| d.to_string()).unwrap_or_default(),
                            r.created_at.to_rfc3339(),
                        ]
                    },
                    &[
                        "id",
                        "channel_id",
                        "user_id",
                        "username",
                        "message_id",
                        "content",
                        "score",
                        "action",
                        "reason",
                        "duration_secs",
                        "created_at",
                    ],
                )
            }
            "audit_logs" => {
                let rows = self.repo.fetch_audit_logs(guild_id, max_rows).await?;
                serialize_rows(
                    &rows,
                    format,
                    |r: &AuditLogExport| {
                        vec![
                            r.id.to_string(),
                            r.event_type.clone(),
                            r.actor_id.clone().unwrap_or_default(),
                            r.actor_name.clone().unwrap_or_default(),
                            r.target_id.clone().unwrap_or_default(),
                            r.target_name.clone().unwrap_or_default(),
                            r.channel_id.clone().unwrap_or_default(),
                            r.channel_name.clone().unwrap_or_default(),
                            r.created_at.to_rfc3339(),
                        ]
                    },
                    &[
                        "id",
                        "event_type",
                        "actor_id",
                        "actor_name",
                        "target_id",
                        "target_name",
                        "channel_id",
                        "channel_name",
                        "created_at",
                    ],
                )
            }
            "moderation_actions" => {
                let rows = self
                    .repo
                    .fetch_moderation_actions(guild_id, max_rows)
                    .await?;
                serialize_rows(
                    &rows,
                    format,
                    |r: &ModerationActionExport| {
                        vec![
                            r.id.to_string(),
                            r.moderator_id.clone(),
                            r.moderator_name.clone(),
                            r.target_id.clone(),
                            r.target_name.clone(),
                            r.action_type.clone(),
                            r.reason.clone(),
                            r.duration.map(|d| d.to_string()).unwrap_or_default(),
                            r.created_at.to_rfc3339(),
                        ]
                    },
                    &[
                        "id",
                        "moderator_id",
                        "moderator_name",
                        "target_id",
                        "target_name",
                        "action_type",
                        "reason",
                        "duration_secs",
                        "created_at",
                    ],
                )
            }
            other => Err(DomainError::ValidationError(format!(
                "job_type inconnu: {other}"
            ))),
        }
    }
}

fn serialize_rows<T, F>(
    rows: &[T],
    format: &str,
    to_csv_row: F,
    headers: &[&str],
) -> Result<ExportResult, DomainError>
where
    T: serde::Serialize,
    F: Fn(&T) -> Vec<String>,
{
    let count = rows.len();
    let data = match format {
        "json" => serde_json::to_string(rows)
            .map_err(|e| DomainError::Internal(format!("json serialize: {e}")))?,
        "csv" => to_csv(rows, headers, to_csv_row),
        other => {
            return Err(DomainError::ValidationError(format!(
                "format inconnu: {other}"
            )))
        }
    };
    Ok(ExportResult {
        data,
        row_count: count,
    })
}

fn to_csv<T, F>(rows: &[T], headers: &[&str], to_row: F) -> String
where
    F: Fn(&T) -> Vec<String>,
{
    let mut out = String::new();
    out.push_str(&headers.join(","));
    out.push('\n');
    for row in rows {
        let line = to_row(row)
            .iter()
            .map(|c| csv_escape(c))
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&line);
        out.push('\n');
    }
    out
}

fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

#[cfg(test)]
#[path = "tests/export_service.rs"]
mod tests;
