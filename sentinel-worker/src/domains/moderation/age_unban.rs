use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use platform_common_worker::is_worker_enabled;

/// Auto-deban des bans de verification d'age arrives a echeance.
///
/// Quand un membre declare un age inferieur au minimum au reglement, il est
/// banni jusqu'a ce qu'il atteigne cet age (`age_verification_bans.unban_at`).
/// Ce job (cadence mensuelle par defaut) leve les bans echus :
///   1. Claim atomique (`FOR UPDATE SKIP LOCKED`) + passage immediat a
///      `status = 'lifted'` -> fire-once (pas de double-deban multi-worker).
///   2. Publie un event `age_ban_lift` via XADD ; le bot (module welcome)
///      consomme et appelle `guild_id.unban(...)` (best-effort, idempotent).
///
/// Le worker n'a pas de connexion gateway : meme pattern XADD->bot consumer
/// que `expire_temp_bans` / `send_reminders`.
#[derive(sqlx::FromRow)]
struct DueAgeBan {
    id: Uuid,
    guild_id: String,
    user_id: String,
}

pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    let due = sqlx::query_as::<_, DueAgeBan>(
        "UPDATE age_verification_bans SET status = 'lifted', lifted_at = NOW()
         WHERE id IN (
             SELECT id FROM age_verification_bans
             WHERE status = 'pending' AND unban_at <= NOW()
             ORDER BY unban_at ASC
             LIMIT 100
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, guild_id, user_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Claim due age bans: {e}"))?;

    if due.is_empty() {
        debug!("Aucun ban d'age a lever");
        return Ok(());
    }

    let mut conn = platform_common_worker::redis_helpers::get_conn(redis).await?;

    for ban in &due {
        if !is_worker_enabled(pool, &ban.guild_id, "welcome-bot").await {
            continue;
        }

        // status deja 'lifted' via le claim atomique ci-dessus (fire-once).
        let payload = serde_json::json!({
            "event": "age_ban_lift",
            "data": {
                "id": ban.id.to_string(),
                "guild_id": ban.guild_id,
                "user_id": ban.user_id,
            }
        });

        if let Err(e) =
            platform_common_worker::redis_helpers::xadd_event_json(&mut conn, &payload).await
        {
            warn!(id = %ban.id, error = %e, "XADD age_ban_lift failed");
        }

        info!(
            id = %ban.id,
            guild_id = %ban.guild_id,
            user_id = %ban.user_id,
            "Ban d'age echu -> event deban emis (Redis)"
        );
    }

    info!(count = due.len(), "Bans d'age echus traites");
    Ok(())
}
