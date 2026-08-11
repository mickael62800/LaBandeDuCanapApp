use redis::AsyncCommands;
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

const REDIS_RESULT_TTL_SECS: u64 = 600;

#[derive(sqlx::FromRow)]
struct AiJobRow {
    id: Uuid,
    guild_id: String,
    job_type: String,
    input_payload: serde_json::Value,
    retries: i32,
    max_retries: i32,
}

/// Phase 4 A — Cycle de drainage de la file ai_jobs.
///
/// Etapes :
///   1. Recuperer les jobs 'processing' qui depassent le timeout et les
///      remettre 'pending' (le worker precedent a probablement crash).
///   2. Atomiquement claim un batch de jobs 'pending' (UPDATE ... RETURNING).
///   3. Pour chaque job : appeler l'API d'inference HTTP, persister le resultat
///      ou l'erreur, publier sur Redis pub/sub `ai_result:{job_id}`.
///
/// Le worker n'embarque PAS les modeles ONNX : il delegue a l'API qui les a
/// deja chargees. Cela simplifie le deploiement (pas de duplication de modeles).
pub async fn run(
    pool: &PgPool,
    redis: &redis::Client,
    api_url: &str,
    job_timeout_secs: u64,
    batch_size: i32,
) -> Result<(), String> {
    // 1. Reset des jobs zombies (processing trop longtemps)
    let resurrected = sqlx::query(
        "UPDATE ai_jobs SET status = 'pending', started_at = NULL \
         WHERE status = 'processing' AND started_at < NOW() - ($1 || ' seconds')::interval",
    )
    .bind(job_timeout_secs.to_string())
    .execute(pool)
    .await
    .map_err(|e| format!("zombie reset: {e}"))?
    .rows_affected();

    if resurrected > 0 {
        warn!(count = resurrected, "Jobs zombies remis en pending");
    }

    // 2. Claim atomique d'un batch
    let claimed: Vec<AiJobRow> = sqlx::query_as::<_, AiJobRow>(
        "UPDATE ai_jobs SET status = 'processing', started_at = NOW() \
         WHERE id IN ( \
             SELECT id FROM ai_jobs \
             WHERE status = 'pending' \
             ORDER BY created_at ASC \
             LIMIT $1 \
             FOR UPDATE SKIP LOCKED \
         ) \
         RETURNING id, guild_id, job_type, input_payload, retries, max_retries",
    )
    .bind(batch_size as i64)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("claim batch: {e}"))?;

    if claimed.is_empty() {
        debug!("Aucun job IA en attente");
        return Ok(());
    }

    info!(count = claimed.len(), "Jobs IA claimes");

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    // 3. Traitement de chaque job
    for job in claimed {
        let job_id = job.id;
        // Guard top-level : si la guild a desactive le module ai, on
        // marque le job comme 'dead' au lieu de le traiter.
        if !platform_common_worker::is_worker_enabled(pool, &job.guild_id, "ai").await {
            let _ = sqlx::query(
                "UPDATE ai_jobs SET status = 'dead', \
                        error_message = 'module ai disabled for guild', \
                        started_at = NULL, completed_at = NOW() \
                 WHERE id = $1",
            )
            .bind(job_id)
            .execute(pool)
            .await;
            continue;
        }
        let result = process_job(&http, api_url, &job).await;

        match result {
            Ok(payload) => {
                if let Err(e) = mark_done(pool, job_id, &payload).await {
                    warn!(job_id = %job_id, error = %e, "mark_done failed");
                }
                publish_result(redis, job_id, &payload).await;
                info!(job_id = %job_id, guild_id = %job.guild_id, kind = %job.job_type, "Job IA termine");
            }
            Err(err) => {
                let new_retries = job.retries + 1;
                let exhausted = new_retries >= job.max_retries;
                let new_status = if exhausted { "dead" } else { "pending" };

                if let Err(e) = sqlx::query(
                    "UPDATE ai_jobs SET status = $1, retries = $2, error_message = $3, \
                            started_at = NULL, completed_at = CASE WHEN $1 = 'dead' THEN NOW() ELSE NULL END \
                     WHERE id = $4",
                )
                .bind(new_status)
                .bind(new_retries)
                .bind(&err)
                .bind(job_id)
                .execute(pool)
                .await
                {
                    warn!(job_id = %job_id, error = %e, "Echec marquage retry");
                }
                warn!(
                    job_id = %job_id,
                    retries = new_retries,
                    exhausted,
                    error = %err,
                    "Job IA en echec"
                );
            }
        }
    }

    Ok(())
}

async fn process_job(
    http: &reqwest::Client,
    api_url: &str,
    job: &AiJobRow,
) -> Result<serde_json::Value, String> {
    let endpoint = match job.job_type.as_str() {
        "analyze_text" => format!("{api_url}/analyze"),
        "analyze_image" => format!("{api_url}/analyze/image"),
        other => return Err(format!("job_type inconnu : {other}")),
    };

    // Note : l'API expose ces endpoints sans /api prefix car ce sont des
    // routes "lourdes" historiques (cf. router.rs heavy_routes).
    let api_key = std::env::var("API_KEY").unwrap_or_default();
    let mut req = http.post(&endpoint).json(&job.input_payload);
    if !api_key.is_empty() {
        req = req.bearer_auth(&api_key);
    }

    let resp = req.send().await.map_err(|e| format!("HTTP send: {e}"))?;

    let status = resp.status();
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("HTTP parse ({status}): {e}"))?;

    if !status.is_success() {
        return Err(format!("API non-success ({status}): {body}"));
    }

    Ok(body)
}

async fn mark_done(pool: &PgPool, job_id: Uuid, payload: &serde_json::Value) -> Result<(), String> {
    sqlx::query(
        "UPDATE ai_jobs SET status = 'done', result_payload = $1, completed_at = NOW() \
         WHERE id = $2",
    )
    .bind(payload)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|e| format!("mark_done: {e}"))?;
    Ok(())
}

async fn publish_result(redis: &redis::Client, job_id: Uuid, payload: &serde_json::Value) {
    let channel = format!("ai_result:{job_id}");
    let key = format!("ai_result:{job_id}");
    let serialized = match serde_json::to_string(payload) {
        Ok(s) => s,
        Err(e) => {
            warn!(error = %e, "publish_result serialize");
            return;
        }
    };

    match redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            if let Err(e) = conn.publish::<_, _, ()>(&channel, &serialized).await {
                tracing::warn!(error = %e, channel, "Echec Redis publish resultat AI");
            }
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&key, &serialized, REDIS_RESULT_TTL_SECS)
                .await
            {
                tracing::warn!(error = %e, key, "Echec Redis set_ex resultat AI");
            }
        }
        Err(e) => {
            tracing::warn!(error = %e, "Echec connexion Redis pour publier resultat AI");
        }
    }
}
