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
    // Une guilde peut avoir desactive l'expulsion automatique : le membre reste
    // alors en attente d'une decision humaine. Sans cette jointure, le reglage
    // s'afficherait a l'ecran sans rien commander, et les gens seraient
    // expulses malgre lui.
    let candidates: Vec<ExpiredQuarantine> = sqlx::query_as(
        "SELECT q.guild_id, q.user_id \
         FROM security_quarantine_pending q \
         LEFT JOIN bot_guild_config k \
           ON k.guild_id = q.guild_id AND k.bot_name = 'security-bot' \
          AND k.config_key = 'quarantine_kick_enabled' \
         WHERE q.expires_at < NOW() \
           AND COALESCE(k.config_value, 'true') IN ('true', '1') \
         ORDER BY q.expires_at ASC LIMIT 100",
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
        if !crate::sentinel::jobs::support::is_enabled(pool, &q.guild_id, "security-bot").await {
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
            crate::sentinel::jobs::support::publish_event(&mut conn, &payload.to_string()).await;
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
