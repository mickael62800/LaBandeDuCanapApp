use chrono::{DateTime, Utc};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use uuid::Uuid;

use platform_common_worker::is_worker_enabled;

#[derive(sqlx::FromRow)]
struct PendingReminder {
    id: Uuid,
    guild_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    expires_at: DateTime<Utc>,
}

/// Phase 4 B (sanction-expiry) — Envoie les rappels de sanctions temporaires.
///
/// Pour chaque rappel `pending` dont `remind_at <= NOW()` :
///   1. Marque `status = 'sent'` AVANT le broadcast pour eviter les doublons.
///   2. Publie un event `sanction_expiry_reminder` via XADD sur la stream
///      `sentinel:events` que le `moderation-bot` consomme via XREADGROUP
///      (Phase 5B — `crate::shared::event_bus::listen_stream_group`).
///   3. Le moderation-bot envoie alors un DM Discord au moderator (acces gateway).
///
/// On ne fait PAS de notification Discord directe ici car le worker n'a pas
/// de connexion gateway. Le pattern XADD→bot consumer est le meme que pour
/// `temp-roles-worker`.
pub async fn run(pool: &PgPool, redis: &redis::Client) -> Result<(), String> {
    // Claim atomique : UPDATE + RETURNING evite la race condition
    // multi-worker (pas de double XADD).
    let reminders = sqlx::query_as::<_, PendingReminder>(
        "UPDATE sanction_reminders SET status = 'sent'
         WHERE id IN (
             SELECT id FROM sanction_reminders
             WHERE status = 'pending' AND remind_at <= NOW()
             ORDER BY remind_at ASC
             LIMIT 50
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, expires_at"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Claim pending reminders: {e}"))?;

    if reminders.is_empty() {
        debug!("Aucun rappel a envoyer");
        return Ok(());
    }

    let mut conn = platform_common_worker::redis_helpers::get_conn(redis).await?;

    for reminder in &reminders {
        if !is_worker_enabled(pool, &reminder.guild_id, "moderation-bot").await {
            continue;
        }

        // Status deja 'sent' via le claim atomique ci-dessus.

        let time_left = reminder.expires_at.signed_duration_since(Utc::now());
        let minutes_left = time_left.num_minutes().max(0);

        // Publier l'event pour que moderation-bot envoie un DM au moderateur
        let payload = serde_json::json!({
            "event": "sanction_expiry_reminder",
            "data": {
                "reminder_id": reminder.id.to_string(),
                "guild_id": reminder.guild_id,
                "moderator_id": reminder.moderator_id,
                "moderator_name": reminder.moderator_name,
                "target_id": reminder.target_id,
                "target_name": reminder.target_name,
                "action_type": reminder.action_type,
                "reason": reminder.reason,
                "expires_at": reminder.expires_at.to_rfc3339(),
                "minutes_left": minutes_left,
            }
        });

        if let Err(e) = platform_common_worker::redis_helpers::xadd_event_json(&mut conn, &payload).await {
            warn!(reminder_id = %reminder.id, error = %e, "XADD reminder failed");
        }

        info!(
            reminder_id = %reminder.id,
            moderator = %reminder.moderator_name,
            target = %reminder.target_name,
            action = %reminder.action_type,
            minutes_left = minutes_left,
            "Rappel de sanction temporaire envoye (Redis)"
        );
    }

    info!(count = reminders.len(), "Rappels de sanctions envoyes");
    Ok(())
}

