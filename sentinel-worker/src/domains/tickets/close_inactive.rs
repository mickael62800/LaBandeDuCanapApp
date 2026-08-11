//! Ferme les tickets inactifs > inactive_close_days (defaut 7j).
//! Pour chaque ticket ferme, XADD un event `ticket_auto_closed` que le
//! bot consume pour faire le menage Discord (notification + delete
//! channel).

use std::collections::HashMap;

use sentinel_core::domain::services::tickets::sla::{
    effective_threshold, is_breached, DEFAULT_INACTIVE_CLOSE_DAYS,
};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct InactiveTicket {
    id: Uuid,
    server: String,
    channel_id: Option<String>,
    inactive_days: i64,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    // Charge les overrides per-guild (inactive_close_days). 1 query.
    let timeouts = load_timeouts(pool).await;

    // Filtre grossier en SQL : tickets non closed, mis a jour il y a au
    // moins 1 jour. Ensuite affinage par config guild.
    let candidates: Vec<InactiveTicket> = sqlx::query_as::<_, InactiveTicket>(
        "SELECT id, server, channel_id, \
                EXTRACT(EPOCH FROM (NOW() - updated_at))::bigint / 86400 AS inactive_days \
         FROM tickets \
         WHERE status != 'closed' \
           AND updated_at < NOW() - INTERVAL '1 day' \
         ORDER BY updated_at ASC \
         LIMIT 200",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query inactive tickets: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucun ticket inactif candidat");
        return Ok(());
    }

    let mut conn = redis.clone();

    let mut closed = 0u32;
    for t in &candidates {
        // Guard : si ticket-bot desactive pour cette guild, on saute.
        if !platform_common_worker::is_worker_enabled(pool, &t.server, "ticket-bot").await {
            continue;
        }
        // Décisions du core : résolution du seuil configuré + breach
        // (seuil <= 0 = désactivé, sinon breach à >= seuil).
        let timeout_days = effective_threshold(
            timeouts.get(&t.server).copied(),
            DEFAULT_INACTIVE_CLOSE_DAYS,
        );
        if !is_breached(t.inactive_days, timeout_days) {
            continue;
        }

        // UPDATE atomique avec garde sur status (idempotence).
        let updated = sqlx::query(
            "UPDATE tickets SET status = 'closed', updated_at = NOW() \
             WHERE id = $1 AND status != 'closed'",
        )
        .bind(t.id)
        .execute(pool)
        .await
        .map_err(|e| format!("close ticket: {e}"))?;
        if updated.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "ticket_auto_closed",
            "data": {
                "ticket_id": t.id.to_string(),
                "guild_id": t.server,
                "channel_id": t.channel_id,
                "inactive_days": t.inactive_days,
                "timeout_days": timeout_days,
            }
        });

        let res =
            platform_common_worker::redis_helpers::xadd_event(&mut conn, &payload.to_string())
                .await;
        if let Err(e) = res {
            warn!(error = %e, ticket_id = %t.id, "XADD ticket_auto_closed echoue");
        }
        closed += 1;
    }

    if closed > 0 {
        info!(closed, "Tickets inactifs fermes -> events publies");
    }
    Ok(())
}

async fn load_timeouts(pool: &PgPool) -> HashMap<String, i64> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT guild_id, config_value FROM bot_guild_config \
         WHERE bot_name = 'ticket-bot' AND config_key = 'inactive_close_days'",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .filter_map(|(g, v)| v.parse::<i64>().ok().map(|n| (g, n)))
        .collect()
}
