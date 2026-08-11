use std::collections::HashMap;

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::{DEFAULT_SLA_ESCALATION_MINUTES, DEFAULT_SLA_FIRST_RESPONSE_MINUTES};

/// Phase 6A — Escalade automatique des appels de sanction en breach de SLA.
///
/// Pour chaque guild avec des tickets d'appel en attente :
///   1. Lit la config SLA depuis `bot_guild_config` (bot_name='ticket-bot') :
///      - `sla_first_response_minutes` (defaut 30)
///      - `sla_escalation_minutes` (defaut 60)
///   2. Scanne les tickets `category='appel_sanction'` avec
///      `status IN ('open','assigned')`, `escalated_at IS NULL`,
///      `first_response_at IS NULL` et `created_at < NOW() - sla_escalation_minutes`
///   3. Marque `escalated_at = NOW()` (evite doublons) AVANT de publier.
///   4. Publie un event `appeal_sla_escalated` via XADD sur `sentinel:events`
///      pour que le moderation-bot ping les moderateurs seniors.
///
/// **Semantique** : on escalade UNE fois par ticket (flag `escalated_at`).
/// Le UPDATE se fait avant l'XADD pour garantir l'idempotence meme si
/// plusieurs instances du worker tournent en parallele.
pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    // Recuperer la config SLA par guild (une seule query)
    let sla_configs = load_sla_configs(pool).await?;

    // Trouver les tickets en breach — on utilise le max(30, 60) = 60min comme
    // filtre grossier, puis on affine par guild avec la config reelle.
    let candidates: Vec<CandidateTicket> = sqlx::query_as::<_, CandidateTicket>(
        "SELECT id, server, author_id, author_name, title, created_at \
         FROM tickets \
         WHERE category = 'appel_sanction' \
           AND status IN ('open', 'assigned') \
           AND escalated_at IS NULL \
           AND first_response_at IS NULL \
           AND created_at < NOW() - INTERVAL '1 minute' \
         ORDER BY created_at ASC \
         LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("query candidate appeals: {e}"))?;

    if candidates.is_empty() {
        debug!("Aucun ticket d'appel en attente d'escalade");
        return Ok(());
    }

    let mut conn = platform_common_worker::redis_helpers::get_conn(redis).await?;

    let now = Utc::now();
    let mut escalated = 0u32;
    let mut skipped = 0u32;

    for ticket in &candidates {
        // Garde per-guild alignée sur les jumeaux tickets/escalate_sla et
        // close_inactive (oubli historique : appeal_sla escaladait même pour
        // les guilds ayant désactivé ticket-bot).
        if !platform_common_worker::is_worker_enabled(pool, &ticket.server, "ticket-bot").await {
            skipped += 1;
            continue;
        }
        let escalation_minutes = sentinel_core::domain::services::tickets::sla::effective_threshold(
            sla_configs
                .get(&ticket.server)
                .map(|c| c.escalation_minutes),
            DEFAULT_SLA_ESCALATION_MINUTES,
        );

        let age_minutes = (now - ticket.created_at).num_minutes();
        if !sentinel_core::domain::services::tickets::sla::is_breached(
            age_minutes,
            escalation_minutes,
        ) {
            skipped += 1;
            continue;
        }

        // UPDATE atomique avec garde WHERE escalated_at IS NULL
        // (prevent double-escalade si plusieurs workers tournent)
        let updated = sqlx::query(
            "UPDATE tickets SET escalated_at = NOW(), updated_at = NOW() \
             WHERE id = $1 AND escalated_at IS NULL",
        )
        .bind(ticket.id)
        .execute(pool)
        .await
        .map_err(|e| format!("mark escalated: {e}"))?;

        if updated.rows_affected() == 0 {
            // Un autre worker nous a devance — skip
            continue;
        }

        let first_response_minutes =
            sentinel_core::domain::services::tickets::sla::effective_threshold(
                sla_configs
                    .get(&ticket.server)
                    .map(|c| c.first_response_minutes),
                DEFAULT_SLA_FIRST_RESPONSE_MINUTES,
            );

        let payload = serde_json::json!({
            "event": "appeal_sla_escalated",
            "data": {
                "ticket_id": ticket.id.to_string(),
                "guild_id": ticket.server,
                "author_id": ticket.author_id,
                "author_name": ticket.author_name,
                "title": ticket.title,
                "created_at": ticket.created_at.to_rfc3339(),
                "age_minutes": age_minutes,
                "sla_first_response_minutes": first_response_minutes,
                "sla_escalation_minutes": escalation_minutes,
            }
        });

        let serialized = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "serialize escalation event");
                continue;
            }
        };

        let res = platform_common_worker::redis_helpers::xadd_event(&mut conn, &serialized).await;

        match res {
            Ok(_) => {
                escalated += 1;
                info!(
                    ticket_id = %ticket.id,
                    guild_id = %ticket.server,
                    age_minutes,
                    "Appel de sanction escalade (SLA breach)"
                );
            }
            Err(e) => warn!(ticket_id = %ticket.id, error = %e, "XADD escalation failed"),
        }
    }

    if escalated > 0 || skipped > 0 {
        info!(
            escalated,
            skipped,
            total = candidates.len(),
            "Scan SLA des appels termine"
        );
    }

    Ok(())
}

#[derive(sqlx::FromRow)]
struct CandidateTicket {
    id: Uuid,
    server: String,
    author_id: String,
    author_name: String,
    title: String,
    created_at: DateTime<Utc>,
}

struct GuildSlaConfig {
    first_response_minutes: i64,
    escalation_minutes: i64,
}

/// Charge la config SLA par guild depuis `bot_guild_config` en une seule query.
///
/// On ne retient que les guilds qui ont explicitement configure au moins une
/// des deux cles — les autres utiliseront les defauts.
async fn load_sla_configs(pool: &PgPool) -> Result<HashMap<String, GuildSlaConfig>, String> {
    #[derive(sqlx::FromRow)]
    struct ConfigRow {
        guild_id: String,
        config_key: String,
        config_value: String,
    }

    let rows: Vec<ConfigRow> = sqlx::query_as::<_, ConfigRow>(
        "SELECT guild_id, config_key, config_value FROM bot_guild_config \
         WHERE bot_name = 'ticket-bot' \
           AND config_key IN ('sla_first_response_minutes', 'sla_escalation_minutes')",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("load SLA configs: {e}"))?;

    let mut map: HashMap<String, GuildSlaConfig> = HashMap::new();
    for row in rows {
        let entry = map.entry(row.guild_id).or_insert(GuildSlaConfig {
            first_response_minutes: DEFAULT_SLA_FIRST_RESPONSE_MINUTES,
            escalation_minutes: DEFAULT_SLA_ESCALATION_MINUTES,
        });
        // Sémantique alignée sur les autres chargeurs SLA (tickets/close_inactive) :
        // valeur configurée stockée BRUTE (un seuil <= 0 = désactivé, tranché par
        // `is_breached`), clé absente ou non numérique = défaut.
        if let Ok(parsed) = row.config_value.parse::<i64>() {
            match row.config_key.as_str() {
                "sla_first_response_minutes" => entry.first_response_minutes = parsed,
                "sla_escalation_minutes" => entry.escalation_minutes = parsed,
                _ => {}
            }
        }
    }

    Ok(map)
}
