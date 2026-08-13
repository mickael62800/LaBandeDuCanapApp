//! Adapter Postgres du port `ExportJobRepository` : file d'attente `export_jobs`.
//! Tout le SQL de l'enqueue/lecture d'un job d'export vit ici (miroir du
//! comportement precedemment inline dans le handler HTTP).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::export_job_repository::{
    ExportJobRecord, ExportJobRepository, NewExportJob,
};

pub struct PgExportJobRepository {
    pool: PgPool,
}

impl PgExportJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ExportJobRow {
    id: Uuid,
    guild_id: String,
    requested_by: String,
    job_type: String,
    format: String,
    status: String,
    result: Option<String>,
    result_rows: Option<i32>,
    error_message: Option<String>,
    retries: i32,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
}

impl From<ExportJobRow> for ExportJobRecord {
    fn from(r: ExportJobRow) -> Self {
        ExportJobRecord {
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

#[async_trait]
impl ExportJobRepository for PgExportJobRepository {
    async fn enqueue(&self, job: &NewExportJob) -> Result<Uuid, DomainError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO export_jobs (guild_id, requested_by, job_type, format, filters) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(&job.guild_id)
        .bind(&job.requested_by)
        .bind(&job.job_type)
        .bind(&job.format)
        .bind(&job.filters)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("export_jobs insert"))?;
        Ok(id)
    }

    async fn find(&self, id: Uuid) -> Result<Option<ExportJobRecord>, DomainError> {
        let row: Option<ExportJobRow> = sqlx::query_as(
            "SELECT id, guild_id, requested_by, job_type, format, status, result, result_rows, \
                    error_message, retries, created_at, started_at, completed_at \
             FROM export_jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("export_jobs select"))?;
        Ok(row.map(ExportJobRecord::from))
    }
}
