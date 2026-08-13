//! Phase 6A — Reconciliation des audit_logs avec l'API Discord.
//!
//! Ce worker importe dans la table `audit_logs` les actions de moderation
//! effectuees HORS de nos bots (via le client Discord directement, ou par
//! un autre bot installe sur la guild). Sans ce sync, le desktop ne voit
//! que les actions effectuees via Sentinel.
//!
//! # Flow
//!
//! 1. Pour chaque guild dans `guilds` :
//!    - Recupere le `last_entry_id` depuis `discord_audit_sync_state`
//!    - Appelle `GET /guilds/{id}/audit-logs?after={last_entry_id}&limit=100`
//!    - Parse les entries et insert dans `audit_logs` avec un prefix
//!      `discord_audit:` sur `event_type` (pour les distinguer des actions
//!      directes)
//!    - Update `last_entry_id` au plus recent fetche
//!
//! # Action types Discord couverts (MVP)
//!
//! - 20 = `MEMBER_KICK`       -> `discord_audit:member_kick`
//! - 22 = `MEMBER_BAN_ADD`    -> `discord_audit:member_ban`
//! - 23 = `MEMBER_BAN_REMOVE` -> `discord_audit:member_unban`
//! - 24 = `MEMBER_UPDATE`     -> `discord_audit:member_timeout` (si timeout)
//! - 25 = `MEMBER_ROLE_UPDATE`-> `discord_audit:member_role_update`
//!
//! Les autres types (channel/role create/delete, message delete, etc.) sont
//! ignores par le MVP — ils peuvent etre ajoutes incrementalement dans
//! `map_action_type`.
//!
//! # Dedup
//!
//! L'`entry_id` Discord vit dans la colonne dediee `discord_entry_id`. Son
//! index unique avec `created_at` rend les relectures idempotentes, y compris
//! apres un reset du curseur. Le lot et le curseur sont commites ensemble.
//!
//! # Rate limits
//!
//! Discord impose un rate limit global + par-route. Pour le MVP on fait 1
//! request par guild par tick (5 min), largement sous le budget. Les
//! headers `X-RateLimit-Remaining` et `Retry-After` ne sont pas encore
//! respectes — a ajouter si on scale a beaucoup de guilds.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::{debug, info, warn};

use super::ENTRIES_PER_CALL;

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

use platform_core::sentinel::domain::services::audit::discord_audit::{
    is_newer_snowflake, map_action_type, snowflake_created_at,
};

#[derive(Debug)]
struct PreparedAuditEntry {
    discord_entry_id: String,
    event_type: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_id: Option<String>,
    details: serde_json::Value,
    created_at: DateTime<Utc>,
}

fn prepare_entries(
    audit_log: &AuditLogResponse,
    last_entry_id: Option<&str>,
) -> Result<(Vec<PreparedAuditEntry>, Option<String>), String> {
    let user_map: HashMap<&str, &str> = audit_log
        .users
        .iter()
        .map(|user| (user.id.as_str(), user.username.as_str()))
        .collect();
    let mut newest_id = last_entry_id.map(str::to_owned);
    let mut entries = Vec::new();

    // Sans curseur Discord renvoie du plus recent au plus ancien ; avec
    // `after`, l'ordre est croissant. La transaction ne depend pas de cet
    // ordre et calcule toujours le maximum numerique pour le curseur.
    for entry in audit_log.audit_log_entries.iter().rev() {
        if is_newer_snowflake(newest_id.as_deref(), &entry.id) {
            newest_id = Some(entry.id.clone());
        }

        let Some(event_type) = map_action_type(entry.action_type) else {
            continue;
        };
        let created_at = snowflake_created_at(&entry.id)
            .ok_or_else(|| format!("snowflake Discord invalide: {}", entry.id))?;
        let actor_name = entry
            .user_id
            .as_deref()
            .and_then(|user_id| user_map.get(user_id).copied())
            .map(str::to_owned);
        let details = serde_json::json!({
            "discord_entry_id": entry.id,
            "action_type_raw": entry.action_type,
            "changes": entry.changes,
            "options": entry.options,
            "reason": entry.reason,
        });

        entries.push(PreparedAuditEntry {
            discord_entry_id: entry.id.clone(),
            event_type,
            actor_id: entry.user_id.clone(),
            actor_name,
            target_id: entry.target_id.clone(),
            details,
            created_at,
        });
    }

    Ok((entries, newest_id))
}

async fn persist_batch(
    pool: &PgPool,
    guild_id: &str,
    entries: &[PreparedAuditEntry],
    newest_id: Option<&str>,
) -> Result<u32, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("begin audit sync transaction: {error}"))?;

    let inserted = if entries.is_empty() {
        0
    } else {
        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO audit_logs \
             (guild_id, discord_entry_id, event_type, actor_id, actor_name, target_id, details, created_at) ",
        );
        query.push_values(entries, |mut row, entry| {
            row.push_bind(guild_id)
                .push_bind(&entry.discord_entry_id)
                .push_bind(&entry.event_type)
                .push_bind(&entry.actor_id)
                .push_bind(&entry.actor_name)
                .push_bind(&entry.target_id)
                .push_bind(&entry.details)
                .push_bind(entry.created_at);
        });
        query.push(
            " ON CONFLICT (discord_entry_id, created_at) \
              WHERE discord_entry_id IS NOT NULL DO NOTHING",
        );
        query
            .build()
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("insert audit batch: {error}"))?
            .rows_affected()
    };

    // Le curseur ne peut jamais regresser si deux instances synchronisent la
    // meme guild en parallele. Il est commite dans la meme transaction que le lot.
    sqlx::query(
        "INSERT INTO discord_audit_sync_state \
             (guild_id, last_entry_id, last_synced_at, last_error, consecutive_errors) \
         VALUES ($1, $2, NOW(), NULL, 0) \
         ON CONFLICT (guild_id) DO UPDATE SET \
            last_entry_id = CASE \
                WHEN EXCLUDED.last_entry_id IS NULL THEN discord_audit_sync_state.last_entry_id \
                WHEN discord_audit_sync_state.last_entry_id IS NULL \
                  OR discord_audit_sync_state.last_entry_id !~ '^[0-9]+$' \
                  OR (EXCLUDED.last_entry_id ~ '^[0-9]+$' AND \
                      EXCLUDED.last_entry_id::numeric > discord_audit_sync_state.last_entry_id::numeric) \
                THEN EXCLUDED.last_entry_id \
                ELSE discord_audit_sync_state.last_entry_id \
            END, \
            last_synced_at = NOW(), \
            last_error = NULL, \
            consecutive_errors = 0",
    )
    .bind(guild_id)
    .bind(newest_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| format!("update sync state: {error}"))?;

    tx.commit()
        .await
        .map_err(|error| format!("commit audit sync transaction: {error}"))?;

    u32::try_from(inserted).map_err(|_| format!("insert count overflow: {inserted}"))
}

pub async fn run(pool: &PgPool, http: &reqwest::Client, bot_token: &str) -> Result<(), String> {
    // 1. Recuperer les guilds a synchroniser
    let guilds: Vec<(String,)> = sqlx::query_as("SELECT guild_id FROM guilds")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("query guilds: {e}"))?;

    if guilds.is_empty() {
        debug!("Aucune guild a synchroniser");
        return Ok(());
    }

    let mut total_imported = 0u32;
    let mut guilds_synced = 0u32;
    let mut guilds_errored = 0u32;

    for (guild_id,) in guilds {
        match sync_guild(http, pool, bot_token, &guild_id).await {
            Ok(imported) => {
                total_imported += imported;
                guilds_synced += 1;
                if imported > 0 {
                    info!(guild_id = %guild_id, imported, "Discord audit log synced");
                }
            }
            Err(e) => {
                warn!(guild_id = %guild_id, error = %e, "Discord audit sync failed");
                guilds_errored += 1;

                // Enregistre l'erreur dans le state.
                if let Err(db_err) = sqlx::query(
                    "INSERT INTO discord_audit_sync_state (guild_id, last_synced_at, last_error, consecutive_errors) \
                     VALUES ($1, NOW(), $2, 1) \
                     ON CONFLICT (guild_id) DO UPDATE SET \
                        last_synced_at = NOW(), \
                        last_error = EXCLUDED.last_error, \
                        consecutive_errors = discord_audit_sync_state.consecutive_errors + 1",
                )
                .bind(&guild_id)
                .bind(&e)
                .execute(pool)
                .await
                {
                    warn!(guild_id = %guild_id, error = %db_err, "Echec sauvegarde error state dans sync_state");
                }
            }
        }
    }

    info!(
        guilds_synced,
        guilds_errored, total_imported, "Discord audit sync tick termine"
    );
    Ok(())
}

async fn sync_guild(
    http: &reqwest::Client,
    pool: &PgPool,
    bot_token: &str,
    guild_id: &str,
) -> Result<u32, String> {
    // 1. Recuperer le curseur
    let last_entry_id: Option<String> = sqlx::query_scalar(
        "SELECT last_entry_id FROM discord_audit_sync_state WHERE guild_id = $1",
    )
    .bind(guild_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("query sync state: {e}"))?
    .flatten();

    // 2. Appel Discord
    let mut url =
        format!("{DISCORD_API_BASE}/guilds/{guild_id}/audit-logs?limit={ENTRIES_PER_CALL}");
    if let Some(ref id) = last_entry_id {
        url.push_str(&format!("&after={id}"));
    }

    let resp = http
        .get(&url)
        .header("Authorization", format!("Bot {bot_token}"))
        .send()
        .await
        .map_err(|e| format!("discord GET failed: {e}"))?;

    let status = resp.status();
    if status == reqwest::StatusCode::FORBIDDEN {
        // Le bot n'a pas VIEW_AUDIT_LOG sur cette guild — on n'insiste pas
        return Err("VIEW_AUDIT_LOG manquant".into());
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        // Rate limited par Discord — respecter Retry-After avant de retenter.
        let retry_after = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(5.0);
        warn!(guild_id = %guild_id, retry_after, "Discord rate limit — attente");
        tokio::time::sleep(std::time::Duration::from_secs_f64(retry_after)).await;
        return Err(format!(
            "rate limited ({retry_after}s), retry au prochain tick"
        ));
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("discord non-success {status}: {body}"));
    }

    let audit_log: AuditLogResponse = resp
        .json()
        .await
        .map_err(|e| format!("discord parse: {e}"))?;

    let (entries, newest_id) = prepare_entries(&audit_log, last_entry_id.as_deref())?;
    persist_batch(pool, guild_id, &entries, newest_id.as_deref()).await
}

// -----------------------------------------------------------------------------
// Discord API response types
// -----------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AuditLogResponse {
    #[serde(default)]
    audit_log_entries: Vec<AuditLogEntry>,
    #[serde(default)]
    users: Vec<DiscordUser>,
}

#[derive(Debug, Deserialize)]
struct AuditLogEntry {
    id: String,
    user_id: Option<String>,
    target_id: Option<String>,
    action_type: u32,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    changes: serde_json::Value,
    #[serde(default)]
    options: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISCORD_EPOCH_MILLIS: u64 = 1_420_070_400_000;

    fn snowflake(millis: u64, sequence: u64) -> String {
        (((millis - DISCORD_EPOCH_MILLIS) << 22) | sequence).to_string()
    }

    fn prepared(entry_id: &str, target_id: &str) -> PreparedAuditEntry {
        PreparedAuditEntry {
            discord_entry_id: entry_id.into(),
            event_type: "discord_audit:member_ban".into(),
            actor_id: Some("actor-1".into()),
            actor_name: Some("Actor".into()),
            target_id: Some(target_id.into()),
            details: serde_json::json!({"discord_entry_id": entry_id}),
            created_at: snowflake_created_at(entry_id).unwrap(),
        }
    }

    #[test]
    fn preparation_keeps_newest_cursor_even_for_unsupported_event() {
        let supported_id = snowflake(1_700_000_000_000, 1);
        let unsupported_id = snowflake(1_700_000_001_000, 2);
        let response = AuditLogResponse {
            audit_log_entries: vec![
                AuditLogEntry {
                    id: unsupported_id.clone(),
                    user_id: None,
                    target_id: None,
                    action_type: 1,
                    reason: None,
                    changes: serde_json::Value::Null,
                    options: serde_json::Value::Null,
                },
                AuditLogEntry {
                    id: supported_id,
                    user_id: Some("actor-1".into()),
                    target_id: Some("target-1".into()),
                    action_type: 22,
                    reason: Some("reason".into()),
                    changes: serde_json::Value::Null,
                    options: serde_json::Value::Null,
                },
            ],
            users: vec![DiscordUser {
                id: "actor-1".into(),
                username: "Actor".into(),
            }],
        };

        let (entries, newest) = prepare_entries(&response, None).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].actor_name.as_deref(), Some("Actor"));
        assert_eq!(newest.as_deref(), Some(unsupported_id.as_str()));
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn replay_is_idempotent_and_cursor_never_regresses(pool: PgPool) -> sqlx::Result<()> {
        let older_id = snowflake(1_700_000_000_000, 1);
        let newer_id = snowflake(1_700_000_001_000, 2);
        let entries = vec![
            prepared(&older_id, "target-1"),
            prepared(&newer_id, "target-2"),
        ];

        let first = persist_batch(&pool, "guild-1", &entries, Some(&newer_id))
            .await
            .unwrap();
        let replay = persist_batch(&pool, "guild-1", &entries, Some(&newer_id))
            .await
            .unwrap();
        persist_batch(&pool, "guild-1", &[], Some(&older_id))
            .await
            .unwrap();

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE guild_id = 'guild-1' \
             AND discord_entry_id IS NOT NULL",
        )
        .fetch_one(&pool)
        .await?;
        let cursor: String = sqlx::query_scalar(
            "SELECT last_entry_id FROM discord_audit_sync_state WHERE guild_id = 'guild-1'",
        )
        .fetch_one(&pool)
        .await?;
        let created_at: DateTime<Utc> =
            sqlx::query_scalar("SELECT created_at FROM audit_logs WHERE discord_entry_id = $1")
                .bind(&older_id)
                .fetch_one(&pool)
                .await?;

        assert_eq!(first, 2);
        assert_eq!(replay, 0);
        assert_eq!(count, 2);
        assert_eq!(cursor, newer_id);
        assert_eq!(created_at.timestamp_millis(), 1_700_000_000_000);
        Ok(())
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn failed_batch_rolls_back_rows_and_cursor(pool: PgPool) -> sqlx::Result<()> {
        sqlx::query(
            "INSERT INTO discord_audit_sync_state (guild_id, last_entry_id) \
             VALUES ('guild-1', '1')",
        )
        .execute(&pool)
        .await?;
        let valid_id = snowflake(1_700_000_000_000, 1);
        let invalid_id = snowflake(1_700_000_001_000, 2);
        let entries = vec![
            prepared(&valid_id, "target-1"),
            prepared(&invalid_id, "target-id-over-20-chars"),
        ];

        let result = persist_batch(&pool, "guild-1", &entries, Some(&invalid_id)).await;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs WHERE guild_id = 'guild-1' \
             AND discord_entry_id IS NOT NULL",
        )
        .fetch_one(&pool)
        .await?;
        let cursor: String = sqlx::query_scalar(
            "SELECT last_entry_id FROM discord_audit_sync_state WHERE guild_id = 'guild-1'",
        )
        .fetch_one(&pool)
        .await?;
        assert!(result.is_err());
        assert_eq!(count, 0);
        assert_eq!(cursor, "1");
        Ok(())
    }
}
