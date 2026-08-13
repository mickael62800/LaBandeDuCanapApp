use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::sentinel::jobs::support::is_enabled as is_worker_enabled;

/// BUG #1/#2 — Auto-unban des bans temporaires a l'expiration.
///
/// Le DM "1h avant" (`send_reminders`) n'a JAMAIS leve de ban : il notifie
/// seulement le moderateur. Ce job est le chemin d'enforcement reel.
///
/// Pour chaque ligne `sanction_reminders` portant un ban temporaire
/// (`action_type LIKE 'ban%'`) dont `expires_at <= NOW()` et `unban_status =
/// 'pending'` :
///   1. Claim atomique multi-worker (`FOR UPDATE SKIP LOCKED`) + passage
///      immediat a `unban_status = 'done'` -> fire-once (pas de double-unban).
///   2. Publie un event `sanction_expired_unban` via XADD ; le moderation-bot
///      consomme et appelle `guild_id.unban(...)` (best-effort).
///
/// Couvre AUSSI les bans courts (<= remind_before) qui n'ont pas de DM : la
/// machine a etats `unban_status` est independante de `status` (DM early).
///
/// Les mutes temporaires utilisent le timeout natif Discord (auto-expire) :
/// ils sont exclus par le filtre `action_type LIKE 'ban%'`.
#[derive(sqlx::FromRow)]
struct ExpiredBan {
    id: Uuid,
    guild_id: String,
    target_id: String,
    target_name: String,
    action_type: String,
    action_id: Uuid,
    expires_at: DateTime<Utc>,
}

pub async fn run(pool: &PgPool, redis: &redis::aio::ConnectionManager) -> Result<(), String> {
    let expired = sqlx::query_as::<_, ExpiredBan>(
        "UPDATE sanction_reminders SET unban_status = 'done'
         WHERE id IN (
             SELECT id FROM sanction_reminders
             WHERE unban_status = 'pending'
               AND action_type LIKE 'ban%'
               AND expires_at <= NOW()
             ORDER BY expires_at ASC
             LIMIT 50
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, guild_id, target_id, target_name, action_type, action_id, expires_at",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Claim expired temp bans: {e}"))?;

    if expired.is_empty() {
        debug!("Aucun ban temporaire a lever");
        return Ok(());
    }

    let mut conn = redis.clone();

    for ban in &expired {
        if !is_worker_enabled(pool, &ban.guild_id, "moderation-bot").await {
            continue;
        }

        // unban_status deja 'done' via le claim atomique ci-dessus (fire-once).
        let payload = serde_json::json!({
            "event": "sanction_expired_unban",
            "data": {
                "reminder_id": ban.id.to_string(),
                "guild_id": ban.guild_id,
                "target_id": ban.target_id,
                "target_name": ban.target_name,
                "action_type": ban.action_type,
                "action_id": ban.action_id.to_string(),
                "expired_at": ban.expires_at.to_rfc3339(),
            }
        });

        if let Err(e) =
            crate::sentinel::jobs::support::publish_event_json(&mut conn, &payload).await
        {
            warn!(reminder_id = %ban.id, error = %e, "XADD sanction_expired_unban failed");
        }

        info!(
            reminder_id = %ban.id,
            guild_id = %ban.guild_id,
            target = %ban.target_name,
            "Ban temporaire expire -> event unban emis (Redis)"
        );
    }

    info!(count = expired.len(), "Bans temporaires expires traites");
    Ok(())
}
