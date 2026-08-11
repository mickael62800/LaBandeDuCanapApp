//! Ban en sursis — enforcement a l'echeance.
//!
//! Pour chaque sursis `status='en_sursis'` dont `expires_at <= NOW()` : claim
//! atomique (`FOR UPDATE SKIP LOCKED` + passage a `status='banni'` -> fire-once),
//! puis publie un event `sursis_ban` (le moderation-bot ban le membre et nettoie
//! le salon d'appel).

use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use platform_common_worker::is_worker_enabled;

#[derive(sqlx::FromRow)]
struct DueSursis {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    reason: String,
    channel_id: Option<String>,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    let due = sqlx::query_as::<_, DueSursis>(
        "UPDATE moderation_sursis SET status = 'banni'
         WHERE id IN (
             SELECT id FROM moderation_sursis
             WHERE status = 'en_sursis' AND expires_at <= NOW()
             ORDER BY expires_at ASC
             LIMIT 50
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, guild_id, user_id, username, reason, channel_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Claim sursis dus: {e}"))?;

    if due.is_empty() {
        debug!("Aucun sursis a bannir");
        return Ok(());
    }

    let mut conn = redis.clone();

    for s in &due {
        if !is_worker_enabled(pool, &s.guild_id, "moderation-bot").await {
            continue;
        }
        let payload = serde_json::json!({
            "event": "sursis_ban",
            "data": {
                "guild_id": s.guild_id,
                "user_id": s.user_id,
                "username": s.username,
                "reason": s.reason,
                "channel_id": s.channel_id,
            }
        });
        if let Err(e) =
            platform_common_worker::redis_helpers::xadd_event_json(&mut conn, &payload).await
        {
            warn!(sursis_id = %s.id, error = %e, "XADD sursis_ban failed");
        }
        info!(sursis_id = %s.id, guild_id = %s.guild_id, target = %s.username, "Sursis expire -> event ban emis");
    }

    info!(count = due.len(), "Sursis expires traites");
    Ok(())
}
