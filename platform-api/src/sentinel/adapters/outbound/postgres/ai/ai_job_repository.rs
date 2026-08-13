//! Adapter sortant Postgres de la file de jobs IA (`ai_jobs`). Tout le SQL du
//! domaine ai_jobs vit ici : enqueue + lecture d'etat.

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::ai::ai_job::{AiJob, NewAiJob};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::ai::ai_job_repository::AiJobRepository;

pub struct PgAiJobRepository {
    pool: PgPool,
}

impl PgAiJobRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AiJobRepository for PgAiJobRepository {
    async fn enqueue(&self, job: &NewAiJob) -> Result<Uuid, DomainError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO ai_jobs (guild_id, job_type, input_payload) \
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(&job.guild_id)
        .bind(&job.job_type)
        .bind(&job.input_payload)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(id)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<AiJob>, DomainError> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                serde_json::Value,
                Option<serde_json::Value>,
                Option<String>,
                i32,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<chrono::DateTime<chrono::Utc>>,
            ),
        >(
            "SELECT id, guild_id, job_type, status, input_payload, result_payload, \
                    error_message, retries, created_at, started_at, completed_at \
             FROM ai_jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(
            |(
                id,
                guild_id,
                job_type,
                status,
                input_payload,
                result_payload,
                error_message,
                retries,
                created_at,
                started_at,
                completed_at,
            )| AiJob {
                id,
                guild_id,
                job_type,
                status,
                input_payload,
                result_payload,
                error_message,
                retries,
                created_at,
                started_at,
                completed_at,
            },
        ))
    }
}
