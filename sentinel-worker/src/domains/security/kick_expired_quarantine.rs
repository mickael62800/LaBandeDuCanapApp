//! Phase 5F — Scanne `security_quarantine_pending` pour les rows
//! expires et publie un event `quarantine_expired` que le bot
//! consume pour kicker.
//!
//! UPDATE+DELETE atomiques avec garde sur expires_at pour idempotence
//! si plusieurs workers tournent.

use sqlx::PgPool;
use tracing::{debug, info, warn};

#[derive(sqlx::FromRow)]
struct ExpiredQuarantine {
    guild_id: String,
    user_id: String,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    let candidates: Vec<ExpiredQuarantine> = sqlx::query_as(
        "SELECT guild_id, user_id \
         FROM security_quarantine_pending \
         WHERE expires_at < NOW() \
         ORDER BY expires_at ASC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query expired quarantine: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucune quarantaine expiree");
        return Ok(());
    }

    let mut conn = redis.clone();

    let mut kicked = 0u32;
    for q in &candidates {
        if !platform_common_worker::is_worker_enabled(pool, &q.guild_id, "security-bot").await {
            continue;
        }
        // Claim atomique : DELETE avec garde sur expires_at. Si une
        // autre instance ou le bot a deja retire l'entree (validation
        // captcha entre-temps), rows_affected = 0, on skip.
        let deleted = sqlx::query(
            "DELETE FROM security_quarantine_pending \
             WHERE guild_id = $1 AND user_id = $2 AND expires_at < NOW()",
        )
        .bind(&q.guild_id)
        .bind(&q.user_id)
        .execute(pool)
        .await
        .map_err(|e| format!("claim expired: {e}"))?;
        if deleted.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "quarantine_expired",
            "data": {
                "guild_id": q.guild_id,
                "user_id": q.user_id,
            }
        });
        let res =
            platform_common_worker::redis_helpers::xadd_event(&mut conn, &payload.to_string())
                .await;
        if let Err(e) = res {
            warn!(error = %e, guild = %q.guild_id, user = %q.user_id, "XADD quarantine_expired echoue");
        }
        kicked += 1;
    }

    if kicked > 0 {
        info!(kicked, "Quarantaines expirees -> events publies pour kick");
    }
    Ok(())
}
