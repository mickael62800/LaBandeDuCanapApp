use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::reminder_repository::ReminderRepository;

pub struct PgReminderRepository {
    pool: PgPool,
}

impl PgReminderRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ReminderRow {
    id: Uuid,
    guild_id: String,
    moderator_id: String,
    moderator_name: String,
    target_id: String,
    target_name: String,
    action_type: String,
    reason: String,
    action_id: Uuid,
    remind_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    status: String,
    created_at: DateTime<Utc>,
}

impl From<ReminderRow> for SanctionReminder {
    fn from(r: ReminderRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id.into(),
            moderator_id: r.moderator_id,
            moderator_name: r.moderator_name,
            target_id: r.target_id,
            target_name: r.target_name,
            action_type: r.action_type,
            reason: r.reason,
            action_id: r.action_id,
            remind_at: r.remind_at,
            expires_at: r.expires_at,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl ReminderRepository for PgReminderRepository {
    async fn save(&self, r: &SanctionReminder) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO sanction_reminders (id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(r.id)
        .bind(r.guild_id.as_str())
        .bind(r.moderator_id.as_str())
        .bind(&r.moderator_name)
        .bind(r.target_id.as_str())
        .bind(&r.target_name)
        .bind(&r.action_type)
        .bind(&r.reason)
        .bind(r.action_id)
        .bind(r.remind_at)
        .bind(r.expires_at)
        .bind(&r.status)
        .bind(r.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("save_reminder"))?;
        Ok(())
    }

    async fn find_pending(&self) -> Result<Vec<SanctionReminder>, DomainError> {
        let rows = sqlx::query_as::<_, ReminderRow>(
            "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at, status, created_at
             FROM sanction_reminders
             WHERE status = 'pending' AND remind_at <= NOW()
             ORDER BY remind_at ASC
             LIMIT 100"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("find_pending_reminders"))?;

        Ok(rows.into_iter().map(SanctionReminder::from).collect())
    }

    async fn mark_sent(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE sanction_reminders SET status = 'sent' WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("mark_sent"))?;
        Ok(())
    }

    async fn cancel_for_action(&self, action_id: Uuid) -> Result<(), DomainError> {
        // BUG #2 : on annule le DM early (`status`) MAIS AUSSI la machine
        // d'auto-unban (`unban_status`). Le worker `expire_temp_bans` claim sur
        // `unban_status = 'pending'` (pas sur `status`) : sans flipper
        // `unban_status`, un ban leve precocement declencherait quand meme un
        // `sanction_expired_unban` tardif a `expires_at`.
        sqlx::query(
            "UPDATE sanction_reminders
             SET status = 'cancelled',
                 unban_status = CASE WHEN unban_status = 'pending' THEN 'cancelled' ELSE unban_status END
             WHERE action_id = $1 AND (status = 'pending' OR unban_status = 'pending')",
        )
        .bind(action_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("cancel_reminders"))?;
        Ok(())
    }

    async fn cancel_for_target(&self, guild_id: &str, target_id: &str) -> Result<u64, DomainError> {
        // Unban manuel precoce : on ne connait que (guild, target). On annule
        // tous les rappels de ban temporaire encore actifs pour cet utilisateur
        // afin que l'auto-unban ne rejoue pas plus tard sur un ban plus recent.
        // Restreint a `action_type LIKE 'ban%'` : les mutes temporaires (timeout
        // natif) ne sont pas concernes par un unban.
        let res = sqlx::query(
            "UPDATE sanction_reminders
             SET status = CASE WHEN status = 'pending' THEN 'cancelled' ELSE status END,
                 unban_status = 'cancelled'
             WHERE guild_id = $1 AND target_id = $2
               AND action_type LIKE 'ban%'
               AND unban_status = 'pending'",
        )
        .bind(guild_id)
        .bind(target_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("cancel_reminders_for_target"))?;
        Ok(res.rows_affected())
    }

    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<SanctionReminder>, DomainError> {
        let rows = sqlx::query_as::<_, ReminderRow>(
            "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, action_id, remind_at, expires_at, status, created_at
             FROM sanction_reminders WHERE guild_id = $1 ORDER BY remind_at DESC LIMIT 50"
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("find_reminders_by_guild"))?;

        Ok(rows.into_iter().map(SanctionReminder::from).collect())
    }
}
