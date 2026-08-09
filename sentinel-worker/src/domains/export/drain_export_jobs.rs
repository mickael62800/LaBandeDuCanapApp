//! Phase 6A — Drain de la file d'attente export_jobs.
//!
//! Flow :
//!   1. Reset les jobs `processing` zombies (> PROCESSING_TIMEOUT_SECS) -> `pending`
//!   2. Claim 1 job via `UPDATE ... FOR UPDATE SKIP LOCKED RETURNING` (atomic)
//!   3. Execute la query selon job_type + serialize selon format
//!   4. UPDATE status = 'done' + result + result_rows (ou 'failed'/'dead' si retry max)
//!
//! Le claim atomique permet de scaler horizontalement l'export-worker sans
//! collision. On traite 1 job par tick pour ne pas bloquer les autres jobs
//! en cas de gros export (next tick dans scan_interval_secs).

use sqlx::PgPool;
use tonic::metadata::MetadataValue;
use tonic::Request;
use tracing::{debug, info, warn};
use uuid::Uuid;

use sentinel_proto::export::v1::export_service_client::ExportServiceClient;
use sentinel_proto::export::v1::ExecuteExportRequest;

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

pub async fn run(
    pool: &PgPool,
    max_rows_per_export: i64,
    processing_timeout_secs: i64,
) -> Result<(), String> {
    // 1. Reset les jobs zombies
    let reset = sqlx::query(
        "UPDATE export_jobs SET status = 'pending', started_at = NULL \
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

    // 2. Claim 1 job pending atomiquement
    let claimed: Option<ClaimedJob> = sqlx::query_as::<_, ClaimedJob>(
        "UPDATE export_jobs SET status = 'processing', started_at = NOW() \
         WHERE id = ( \
             SELECT id FROM export_jobs \
             WHERE status = 'pending' \
             ORDER BY created_at ASC \
             FOR UPDATE SKIP LOCKED \
             LIMIT 1 \
         ) \
         RETURNING id, guild_id, job_type, format, filters, retries, max_retries",
    )
    .fetch_optional(pool)
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
    if !platform_common_worker::is_worker_enabled(pool, &job.guild_id, "export").await {
        let _ = sqlx::query(
            "UPDATE export_jobs SET status = 'dead', \
                    error_message = 'module export disabled for guild', \
                    started_at = NULL, completed_at = NOW() \
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
            sqlx::query(
                "UPDATE export_jobs SET status = 'done', result = $1, result_rows = $2, completed_at = NOW() \
                 WHERE id = $3",
            )
            .bind(&serialized)
            .bind(rows as i32)
            .bind(job.id)
            .execute(pool)
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
            let new_retries = job.retries + 1;
            let dead = new_retries >= job.max_retries;
            let new_status = if dead { "dead" } else { "failed" };

            sqlx::query(
                "UPDATE export_jobs SET status = $1, retries = $2, error_message = $3, completed_at = NOW() \
                 WHERE id = $4",
            )
            .bind(new_status)
            .bind(new_retries)
            .bind(&err)
            .bind(job.id)
            .execute(pool)
            .await
            .map_err(|e| format!("mark failed: {e}"))?;

            warn!(job_id = %job.id, error = %err, retries = new_retries, dead, "Export job failed");
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
    let api_key = std::env::var("API_KEY").unwrap_or_default();

    // Delegue a platform_common_worker::grpc::connect() pour beneficier
    // du mTLS optionnel (GRPC_TLS_DIR) en coherence avec les autres callers.
    let channel = platform_common_worker::grpc::connect().await?;

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

