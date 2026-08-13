//! Port outbound : file d'attente des jobs d'export (`export_jobs`).
//! Le POST enqueue un job (202), le GET lit son statut/resultat. Tout le SQL
//! vit dans l'adapter Postgres ; le use case ne manipule que ces DTOs purs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::errors::DomainError;

/// Demande d'enqueue d'un job d'export (INSERT).
#[derive(Debug, Clone)]
pub struct NewExportJob {
    pub guild_id: String,
    pub requested_by: String,
    /// "infractions" | "audit_logs" | "moderation_actions"
    pub job_type: String,
    /// "csv" | "json"
    pub format: String,
    pub filters: serde_json::Value,
}

/// Etat complet d'un job d'export tel que persiste (lu par le GET).
#[derive(Debug, Clone)]
pub struct ExportJobRecord {
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
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait ExportJobRepository: Send + Sync {
    /// Enfile un nouveau job et retourne son id genere.
    async fn enqueue(&self, job: &NewExportJob) -> Result<Uuid, DomainError>;

    /// Recupere l'etat d'un job par son id (None si inexistant).
    async fn find(&self, id: Uuid) -> Result<Option<ExportJobRecord>, DomainError>;
}
