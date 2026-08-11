//! Phase 5I — Escalade SLA pour TOUS les tickets (sauf appel_sanction
//! qui est gere par appeal_sla::escalate_appeal_sla).
//!
//! Avant : tickets/mod.rs avait une boucle 5min qui scannait l'API
//! tickets et utilisait un SlaTracker RAM. Si le bot redemarrait,
//! les timestamps de premiere reponse etaient perdus.
//!
//! Maintenant : la donnee est deja en DB (tickets.first_response_at),
//! le worker scanne et publie un event ticket_sla_escalated. Le bot
//! consume et poste le message d'avertissement dans le channel.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sentinel_core::domain::services::tickets::sla::{
    effective_threshold, is_breached, DEFAULT_SLA_ESCALATION_MINUTES,
    DEFAULT_SLA_FIRST_RESPONSE_MINUTES as DEFAULT_SLA_WARN_MINUTES,
};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct CandidateTicket {
    id: Uuid,
    server: String,
    channel_id: Option<String>,
    created_at: DateTime<Utc>,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    let timeouts = load_escalation_timeouts(pool).await;
    let warn_thresholds = load_warn_thresholds(pool).await;

    // ── Phase warning : tickets pas encore repondus ni warned, age >=
    // sla_first_response_minutes -> publish ticket_sla_warned + UPDATE.
    if let Err(e) = scan_and_warn(pool, redis, &warn_thresholds).await {
        warn!(error = %e, "Erreur scan SLA warning");
    }

    // Tickets pas encore repondus + pas encore escalades + categorie != appel.
    // Filtre grossier > 1 min, on affine par guild apres.
    let candidates: Vec<CandidateTicket> = sqlx::query_as(
        "SELECT id, server, channel_id, created_at \
         FROM tickets \
         WHERE category != 'appel_sanction' \
           AND status IN ('open', 'assigned') \
           AND escalated_at IS NULL \
           AND first_response_at IS NULL \
           AND created_at < NOW() - INTERVAL '1 minute' \
         ORDER BY created_at ASC \
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query candidate tickets: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucun ticket non-appel en attente d'escalade");
        return Ok(());
    }

    let mut conn = redis.clone();

    let now = Utc::now();
    let mut escalated = 0u32;

    for t in &candidates {
        if !platform_common_worker::is_worker_enabled(pool, &t.server, "ticket-bot").await {
            continue;
        }
        let escalation_minutes = effective_threshold(
            timeouts.get(&t.server).copied(),
            DEFAULT_SLA_ESCALATION_MINUTES,
        );
        let age_minutes = (now - t.created_at).num_minutes();
        if !is_breached(age_minutes, escalation_minutes) {
            continue;
        }

        // UPDATE atomique avec garde. On MONTE la priorite a 'high' SANS
        // retrograder un ticket deja 'urgent' (le staff l'avait manuellement
        // eleve) -> l'escalade ne doit jamais baisser la priorite.
        let updated = sqlx::query(
            "UPDATE tickets SET escalated_at = NOW(), updated_at = NOW(), \
                 priority = CASE WHEN priority = 'urgent' THEN 'urgent' ELSE 'high' END \
             WHERE id = $1 AND escalated_at IS NULL",
        )
        .bind(t.id)
        .execute(pool)
        .await
        .map_err(|e| format!("mark escalated: {e}"))?;
        if updated.rows_affected() == 0 {
            continue;
        }

        let payload = serde_json::json!({
            "event": "ticket_sla_escalated",
            "data": {
                "ticket_id": t.id.to_string(),
                "guild_id": t.server,
                "channel_id": t.channel_id,
                "age_minutes": age_minutes,
                "escalation_minutes": escalation_minutes,
            }
        });
        let res =
            platform_common_worker::redis_helpers::xadd_event(&mut conn, &payload.to_string())
                .await;
        if let Err(e) = res {
            warn!(error = %e, ticket_id = %t.id, "XADD ticket_sla_escalated echoue");
        }
        escalated += 1;
    }

    if escalated > 0 {
        info!(escalated, "Tickets escalades SLA -> events publies");
    }
    Ok(())
}

async fn load_escalation_timeouts(pool: &PgPool) -> HashMap<String, i64> {
    load_int_config(pool, "sla_escalation_minutes").await
}

async fn load_warn_thresholds(pool: &PgPool) -> HashMap<String, i64> {
    load_int_config(pool, "sla_first_response_minutes").await
}

async fn load_int_config(pool: &PgPool, key: &str) -> HashMap<String, i64> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT guild_id, config_value FROM bot_guild_config \
         WHERE bot_name = 'ticket-bot' AND config_key = $1",
    )
    .bind(key)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    rows.into_iter()
        .filter_map(|(g, v)| v.parse::<i64>().ok().map(|n| (g, n)))
        .collect()
}

async fn scan_and_warn(
    pool: &PgPool,
    redis: &redis::aio::ConnectionManager,
    thresholds: &HashMap<String, i64>,
) -> Result<(), String> {
    let candidates: Vec<CandidateTicket> = sqlx::query_as(
        "SELECT id, server, channel_id, created_at \
         FROM tickets \
         WHERE category != 'appel_sanction' \
           AND status IN ('open', 'assigned') \
           AND first_response_at IS NULL \
           AND sla_warned_at IS NULL \
           AND escalated_at IS NULL \
           AND created_at < NOW() - INTERVAL '1 minute' \
         ORDER BY created_at ASC \
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query warn candidates: {e}"))?;

    if candidates.is_empty() {
        return Ok(());
    }
    let mut conn = redis.clone();
    let now = Utc::now();
    let mut warned = 0u32;
    for t in &candidates {
        if !platform_common_worker::is_worker_enabled(pool, &t.server, "ticket-bot").await {
            continue;
        }
        let warn_minutes =
            effective_threshold(thresholds.get(&t.server).copied(), DEFAULT_SLA_WARN_MINUTES);
        let age_minutes = (now - t.created_at).num_minutes();
        if !is_breached(age_minutes, warn_minutes) {
            continue;
        }
        // Claim atomique : marque sla_warned_at avec garde.
        let updated = sqlx::query(
            "UPDATE tickets SET sla_warned_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND sla_warned_at IS NULL AND first_response_at IS NULL",
        )
        .bind(t.id)
        .execute(pool)
        .await
        .map_err(|e| format!("mark warned: {e}"))?;
        if updated.rows_affected() == 0 {
            continue;
        }
        let payload = serde_json::json!({
            "event": "ticket_sla_warned",
            "data": {
                "ticket_id": t.id.to_string(),
                "guild_id": t.server,
                "channel_id": t.channel_id,
                "age_minutes": age_minutes,
                "warn_minutes": warn_minutes,
            }
        });
        let res =
            platform_common_worker::redis_helpers::xadd_event(&mut conn, &payload.to_string())
                .await;
        if let Err(e) = res {
            warn!(error = %e, ticket_id = %t.id, "XADD ticket_sla_warned echoue");
        }
        warned += 1;
    }
    if warned > 0 {
        info!(warned, "Tickets SLA warned -> events publies");
    }
    Ok(())
}
