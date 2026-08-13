//! Phase 6A — Drain de la file d'attente export_jobs.
//!
//! Flow :
//!   1. Reset les jobs `processing` zombies (> PROCESSING_TIMEOUT_SECS) -> `pending`
//!   2. Claim 1 job eligible via `UPDATE ... FOR UPDATE SKIP LOCKED RETURNING` (atomic)
//!   3. Execute la query selon job_type + serialize selon format
//!   4. UPDATE status = 'done', `pending` avec backoff, ou `dead` si retries epuises
//!
//! Le claim atomique permet de scaler horizontalement l'export-worker sans
//! collision. On traite 1 job par tick pour ne pas bloquer les autres jobs
//! en cas de gros export (next tick dans scan_interval_secs).

use sqlx::PgPool;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, info, warn};
use uuid::Uuid;

use platform_proto::sentinel::export::v1::export_service_client::ExportServiceClient;
use platform_proto::sentinel::export::v1::ExecuteExportRequest;

const RETRY_BACKOFF_BASE_SECS: i64 = 5;
const RETRY_BACKOFF_MAX_SECS: i64 = 300;

#[derive(Debug, sqlx::FromRow)]
struct ClaimedJob {
    id: Uuid,
    guild_id: String,
    job_type: String,
    format: String,
    filters: serde_json::Value,
    retries: i32,
    max_retries: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureDisposition {
    Retry { retries: i32, delay_secs: i64 },
    Dead { retries: i32 },
}

fn retry_backoff_secs(retries: i32) -> i64 {
    let exponent = retries.saturating_sub(1).clamp(0, 30) as u32;
    RETRY_BACKOFF_BASE_SECS
        .saturating_mul(2_i64.saturating_pow(exponent))
        .min(RETRY_BACKOFF_MAX_SECS)
}

fn failure_disposition(retries: i32, max_retries: i32) -> FailureDisposition {
    let retries = retries.saturating_add(1);
    if retries >= max_retries {
        FailureDisposition::Dead { retries }
    } else {
        FailureDisposition::Retry {
            retries,
            delay_secs: retry_backoff_secs(retries),
        }
    }
}

async fn claim_next_job(pool: &PgPool) -> Result<Option<ClaimedJob>, sqlx::Error> {
    sqlx::query_as::<_, ClaimedJob>(
        "UPDATE export_jobs \
         SET status = 'processing', started_at = NOW(), completed_at = NULL, next_attempt_at = NULL \
         WHERE id = ( \
             SELECT id FROM export_jobs \
             WHERE status = 'pending' \
               AND COALESCE(next_attempt_at, created_at) <= NOW() \
             ORDER BY COALESCE(next_attempt_at, created_at) ASC, created_at ASC \
             FOR UPDATE SKIP LOCKED \
             LIMIT 1 \
         ) \
         RETURNING id, guild_id, job_type, format, filters, retries, max_retries",
    )
    .fetch_optional(pool)
    .await
}

async fn mark_done(
    pool: &PgPool,
    job_id: Uuid,
    serialized: &str,
    rows: usize,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE export_jobs \
         SET status = 'done', result = $1, result_rows = $2, error_message = NULL, \
             next_attempt_at = NULL, completed_at = NOW() \
         WHERE id = $3",
    )
    .bind(serialized)
    .bind(rows as i32)
    .bind(job_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_failure(
    pool: &PgPool,
    job: &ClaimedJob,
    error: &str,
) -> Result<FailureDisposition, sqlx::Error> {
    let disposition = failure_disposition(job.retries, job.max_retries);
    match disposition {
        FailureDisposition::Retry {
            retries,
            delay_secs,
        } => {
            sqlx::query(
                "UPDATE export_jobs \
                 SET status = 'pending', retries = $1, error_message = $2, started_at = NULL, \
                     completed_at = NULL, next_attempt_at = NOW() + make_interval(secs => $3) \
                 WHERE id = $4",
            )
            .bind(retries)
            .bind(error)
            .bind(delay_secs)
            .bind(job.id)
            .execute(pool)
            .await?;
        }
        FailureDisposition::Dead { retries } => {
            sqlx::query(
                "UPDATE export_jobs \
                 SET status = 'dead', retries = $1, error_message = $2, started_at = NULL, \
                     completed_at = NOW(), next_attempt_at = NULL \
                 WHERE id = $3",
            )
            .bind(retries)
            .bind(error)
            .bind(job.id)
            .execute(pool)
            .await?;
        }
    }
    Ok(disposition)
}

pub async fn run(
    pool: &PgPool,
    max_rows_per_export: i64,
    processing_timeout_secs: i64,
) -> Result<(), String> {
    // 1. Reset les jobs zombies
    let reset = sqlx::query(
        "UPDATE export_jobs SET status = 'pending', started_at = NULL, next_attempt_at = NULL \
         WHERE status = 'processing' \
           AND started_at < NOW() - make_interval(secs => $1)",
    )
    .bind(processing_timeout_secs)
    .execute(pool)
    .await
    .map_err(|e| format!("reset zombies: {e}"))?;
    if reset.rows_affected() > 0 {
        warn!(count = reset.rows_affected(), "Export jobs zombies reset");
    }

    // 2. Claim 1 job pending dont le backoff est ecoule, atomiquement.
    let claimed = claim_next_job(pool)
        .await
        .map_err(|e| format!("claim job: {e}"))?;

    let Some(job) = claimed else {
        debug!("Aucun export job pending");
        return Ok(());
    };

    info!(
        job_id = %job.id,
        guild_id = %job.guild_id,
        job_type = %job.job_type,
        format = %job.format,
        "Export job claim"
    );

    // Guard top-level : si la guild a desactive le module export, on
    // marque le job comme 'dead' (le user devra reactiver pour rejouer).
    if !crate::sentinel::jobs::support::is_enabled(pool, &job.guild_id, "export").await {
        let _ = sqlx::query(
            "UPDATE export_jobs SET status = 'dead', \
                    error_message = 'module export disabled for guild', \
                    started_at = NULL, completed_at = NOW(), next_attempt_at = NULL \
             WHERE id = $1",
        )
        .bind(job.id)
        .execute(pool)
        .await;
        return Ok(());
    }

    // 3. Appel gRPC a l'API pour executer l'export (zero logique metier ici).
    let result = call_export_api(
        &job.guild_id,
        &job.job_type,
        &job.format,
        &job.filters,
        max_rows_per_export,
    )
    .await;

    // 4. Persister le resultat
    match result {
        Ok((serialized, rows)) => {
            mark_done(pool, job.id, &serialized, rows)
                .await
                .map_err(|e| format!("mark done: {e}"))?;

            info!(
                job_id = %job.id,
                rows,
                bytes = serialized.len(),
                "Export job done"
            );
        }
        Err(err) => {
            let disposition = mark_failure(pool, &job, &err)
                .await
                .map_err(|e| format!("mark failed: {e}"))?;
            match disposition {
                FailureDisposition::Retry {
                    retries,
                    delay_secs,
                } => warn!(
                    job_id = %job.id,
                    error = %err,
                    retries,
                    delay_secs,
                    "Export job en echec transitoire, retry planifie"
                ),
                FailureDisposition::Dead { retries } => warn!(
                    job_id = %job.id,
                    error = %err,
                    retries,
                    "Export job dead apres epuisement des retries"
                ),
            }
        }
    }

    Ok(())
}

/// Appelle l'API gRPC ExportService.ExecuteExport.
async fn call_export_api(
    guild_id: &str,
    job_type: &str,
    format: &str,
    filters: &serde_json::Value,
    max_rows: i64,
) -> Result<(String, usize), String> {
    // `SENTINEL_API_KEY` : seul nom defini par le compose pour les workers.
    // Sous `API_KEY`, la cle etait vide et l'interceptor gRPC de l'API
    // repondait `unauthenticated` — tous les exports finissaient en `dead`.
    let api_key = std::env::var("SENTINEL_API_KEY").unwrap_or_default();

    // Delegue a crate::sentinel::grpc::connect() pour beneficier du mTLS optionnel
    // (GRPC_TLS_DIR) en coherence avec les autres callers.
    let channel = super::grpc::connect().await?;

    let mut client = ExportServiceClient::new(channel);
    let mut req = Request::new(ExecuteExportRequest {
        guild_id: guild_id.to_string(),
        job_type: job_type.to_string(),
        format: format.to_string(),
        filters_json: filters.to_string(),
        max_rows,
    });
    if let Ok(v) = format!("Bearer {api_key}").parse::<MetadataValue<_>>() {
        req.metadata_mut().insert("authorization", v);
    }

    let resp = client
        .execute_export(req)
        .await
        .map_err(|e| format!("gRPC ExecuteExport: {e}"))?
        .into_inner();

    Ok((resp.data, resp.row_count as usize))
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;

    #[derive(sqlx::FromRow)]
    struct RetryState {
        status: String,
        retries: i32,
        next_attempt_at: Option<DateTime<Utc>>,
        started_at: Option<DateTime<Utc>>,
        completed_at: Option<DateTime<Utc>>,
    }

    async fn insert_job(pool: &PgPool, retries: i32, max_retries: i32) -> sqlx::Result<Uuid> {
        sqlx::query_scalar(
            "INSERT INTO export_jobs \
                 (guild_id, requested_by, job_type, format, retries, max_retries) \
             VALUES ('1', '2', 'audit_logs', 'json', $1, $2) \
             RETURNING id",
        )
        .bind(retries)
        .bind(max_retries)
        .fetch_one(pool)
        .await
    }

    async fn load_claimed_job(pool: &PgPool, id: Uuid) -> sqlx::Result<ClaimedJob> {
        sqlx::query_as(
            "SELECT id, guild_id, job_type, format, filters, retries, max_retries \
             FROM export_jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_one(pool)
        .await
    }

    #[test]
    fn backoff_is_exponential_and_capped() {
        assert_eq!(retry_backoff_secs(1), 5);
        assert_eq!(retry_backoff_secs(2), 10);
        assert_eq!(retry_backoff_secs(3), 20);
        assert_eq!(retry_backoff_secs(7), 300);
        assert_eq!(retry_backoff_secs(30), 300);
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn successful_export_is_marked_done(pool: PgPool) -> sqlx::Result<()> {
        let id = insert_job(&pool, 0, 3).await?;

        mark_done(&pool, id, "[{\"id\":1}]", 1).await?;

        let row: (String, Option<String>, Option<i32>, Option<String>) = sqlx::query_as(
            "SELECT status, result, result_rows, error_message FROM export_jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(
            row,
            ("done".into(), Some("[{\"id\":1}]".into()), Some(1), None)
        );
        Ok(())
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn transient_failure_returns_to_pending_with_backoff(pool: PgPool) -> sqlx::Result<()> {
        let id = insert_job(&pool, 0, 3).await?;
        let job = load_claimed_job(&pool, id).await?;

        let disposition = mark_failure(&pool, &job, "temporary").await?;

        assert_eq!(
            disposition,
            FailureDisposition::Retry {
                retries: 1,
                delay_secs: 5
            }
        );
        let row: RetryState = sqlx::query_as(
            "SELECT status, retries, next_attempt_at, started_at, completed_at \
             FROM export_jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(row.status, "pending");
        assert_eq!(row.retries, 1);
        assert!(row.next_attempt_at.is_some());
        assert!(row.started_at.is_none());
        assert!(row.completed_at.is_none());
        Ok(())
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn exhausted_failure_is_marked_dead(pool: PgPool) -> sqlx::Result<()> {
        let id = insert_job(&pool, 2, 3).await?;
        let job = load_claimed_job(&pool, id).await?;

        let disposition = mark_failure(&pool, &job, "permanent").await?;

        assert_eq!(disposition, FailureDisposition::Dead { retries: 3 });
        let row: RetryState = sqlx::query_as(
            "SELECT status, retries, next_attempt_at, started_at, completed_at \
                 FROM export_jobs WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await?;
        assert_eq!(row.status, "dead");
        assert_eq!(row.retries, 3);
        assert!(row.next_attempt_at.is_none());
        assert!(row.started_at.is_none());
        assert!(row.completed_at.is_some());
        Ok(())
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn claim_skips_job_until_backoff_expires(pool: PgPool) -> sqlx::Result<()> {
        let delayed_id = insert_job(&pool, 1, 3).await?;
        sqlx::query(
            "UPDATE export_jobs SET next_attempt_at = NOW() + INTERVAL '1 hour' WHERE id = $1",
        )
        .bind(delayed_id)
        .execute(&pool)
        .await?;
        let ready_id = insert_job(&pool, 0, 3).await?;

        let claimed = claim_next_job(&pool).await?.expect("un job eligible");

        assert_eq!(claimed.id, ready_id);
        let delayed_status: String =
            sqlx::query_scalar("SELECT status FROM export_jobs WHERE id = $1")
                .bind(delayed_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(delayed_status, "pending");
        Ok(())
    }
}
