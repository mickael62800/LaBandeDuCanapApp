use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::action::applied::ModerationAction;
use platform_core::sentinel::domain::enums::moderation::moderation_gravity::ModerationGravity;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::moderation_repository::ModerationRepository;

/// Phase 2 helper : reconstruit une ModerationAction a partir d'une ligne
/// audit_logs (event_type `mod_*`).
#[derive(sqlx::FromRow)]
struct AuditModRow {
    id: Uuid,
    guild_id: String,
    event_type: String,
    actor_id: Option<String>,
    actor_name: Option<String>,
    target_id: Option<String>,
    target_name: Option<String>,
    /// Lu uniquement par les queries qui font le LEFT JOIN guild_members.
    /// Default = None pour les autres queries (l'aliasing est explicite cote SQL).
    #[sqlx(default)]
    target_display_name: Option<String>,
    channel_id: Option<String>,
    details: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditModRow> for ModerationAction {
    fn from(row: AuditModRow) -> Self {
        let action_type = ModerationAction::action_type_from_audit_event(&row.event_type)
            .unwrap_or(&row.event_type)
            .to_string();
        let reason = row
            .details
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let gravity = row
            .details
            .get("gravity")
            .and_then(|v| v.as_str())
            .and_then(ModerationGravity::from_str_lossy);
        // Negative duration → None (ne wrap pas sur u64::MAX).
        let duration = row
            .details
            .get("duration_secs")
            .and_then(|v| v.as_i64())
            .and_then(|d| u64::try_from(d).ok());
        // Si details.action_id existe, on l'utilise pour conserver l'identite
        // historique (Phase 4 : sera l'id audit_log lui-meme).
        let id = row
            .details
            .get("action_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::from_str(s).ok())
            .unwrap_or(row.id);
        Self {
            id,
            guild_id: row.guild_id.into(),
            channel_id: row.channel_id.unwrap_or_default().into(),
            moderator_id: row.actor_id.unwrap_or_default(),
            moderator_name: row.actor_name.unwrap_or_default(),
            target_id: row.target_id.unwrap_or_default(),
            target_name: row.target_name.unwrap_or_default(),
            target_display_name: row.target_display_name,
            action_type,
            reason,
            gravity,
            duration,
            created_at: row.created_at,
        }
    }
}

const AUDIT_MOD_SELECT: &str =
    "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at FROM audit_logs";

pub struct PgModerationRepository {
    pool: PgPool,
}

impl PgModerationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ModerationRepository for PgModerationRepository {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO audit_logs \
                 (id, guild_id, event_type, actor_id, actor_name, target_id, target_name, \
                  channel_id, channel_name, details, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, $10)",
        )
        .bind(action.id)
        .bind(action.guild_id.as_str())
        .bind(action.audit_event_type())
        .bind(&action.moderator_id)
        .bind(&action.moderator_name)
        .bind(&action.target_id)
        .bind(&action.target_name)
        .bind(action.channel_id.as_str())
        .bind(action.audit_details())
        .bind(action.created_at)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("insert moderation audit log"))?;
        Ok(())
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<ModerationAction>, DomainError> {
        let row = sqlx::query_as::<_, AuditModRow>(
            "SELECT id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at \
             FROM audit_logs \
             WHERE event_type LIKE 'mod_%' AND id = $1 \
             LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(row.map(ModerationAction::from))
    }

    async fn find_by_target(
        &self,
        guild_id: &str,
        target_id: &str,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        let limit = limit.clamp(1, 1000);
        let sql = format!(
            "{AUDIT_MOD_SELECT} WHERE guild_id = $1 AND target_id = $2 AND event_type LIKE 'mod_%' ORDER BY created_at DESC LIMIT {limit}"
        );
        let rows = sqlx::query_as::<_, AuditModRow>(&sql)
            .bind(guild_id)
            .bind(target_id)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn find_bans(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        // Phase 2 : lecture depuis audit_logs.
        // Pour chaque (guild_id, target_id), on prend la derniere action ban*/unban
        // et on ne garde que celles dont l'event_type final commence par 'mod_ban'.
        // LEFT JOIN guild_members pour enrichir avec le pseudo serveur
        // (target_display_name) — affiche dans la liste "Bannis actifs" cote web.
        let rows = match guild_id {
            Some(gid) => {
                sqlx::query_as::<_, AuditModRow>(
                    "SELECT latest.id, latest.guild_id, latest.event_type, latest.actor_id, latest.actor_name, \
                            latest.target_id, latest.target_name, gm.display_name AS target_display_name, \
                            latest.channel_id, latest.details, latest.created_at \
                     FROM ( \
                        SELECT DISTINCT ON (guild_id, target_id) \
                            id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at \
                        FROM audit_logs \
                        WHERE guild_id = $1 \
                          AND target_id IS NOT NULL \
                          AND (event_type LIKE 'mod_ban%' OR event_type = 'mod_unban') \
                        ORDER BY guild_id, target_id, created_at DESC \
                     ) latest \
                     LEFT JOIN guild_members gm \
                         ON gm.guild_id = latest.guild_id AND gm.user_id = latest.target_id \
                     WHERE latest.event_type LIKE 'mod_ban%' \
                     ORDER BY latest.created_at DESC \
                     LIMIT $2 OFFSET $3",
                )
                .bind(gid)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
            None => {
                sqlx::query_as::<_, AuditModRow>(
                    "SELECT latest.id, latest.guild_id, latest.event_type, latest.actor_id, latest.actor_name, \
                            latest.target_id, latest.target_name, gm.display_name AS target_display_name, \
                            latest.channel_id, latest.details, latest.created_at \
                     FROM ( \
                        SELECT DISTINCT ON (guild_id, target_id) \
                            id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, details, created_at \
                        FROM audit_logs \
                        WHERE target_id IS NOT NULL \
                          AND (event_type LIKE 'mod_ban%' OR event_type = 'mod_unban') \
                        ORDER BY guild_id, target_id, created_at DESC \
                     ) latest \
                     LEFT JOIN guild_members gm \
                         ON gm.guild_id = latest.guild_id AND gm.user_id = latest.target_id \
                     WHERE latest.event_type LIKE 'mod_ban%' \
                     ORDER BY latest.created_at DESC \
                     LIMIT $1 OFFSET $2",
                )
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
            }
        }
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn find_all_for_guild(
        &self,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        // Phase 2 : lecture depuis audit_logs.
        let rows = match guild_id {
            Some(gid) => {
                let sql = format!(
                    "{AUDIT_MOD_SELECT} WHERE guild_id = $1 AND event_type LIKE 'mod_%' ORDER BY created_at DESC LIMIT $2"
                );
                sqlx::query_as::<_, AuditModRow>(&sql)
                    .bind(gid)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
            }
            None => {
                let sql = format!(
                    "{AUDIT_MOD_SELECT} WHERE event_type LIKE 'mod_%' ORDER BY created_at DESC LIMIT $1"
                );
                sqlx::query_as::<_, AuditModRow>(&sql)
                    .bind(limit)
                    .fetch_all(&self.pool)
                    .await
            }
        }
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(ModerationAction::from).collect())
    }

    async fn delete_bans_for_user(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        // Phase 4 : on supprime depuis audit_logs.
        sqlx::query(
            "DELETE FROM audit_logs WHERE guild_id = $1 AND target_id = $2 AND event_type LIKE 'mod_ban%'",
        )
        .bind(guild_id)
        .bind(target_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError> {
        let result =
            sqlx::query("DELETE FROM audit_logs WHERE event_type LIKE 'mod_%' AND id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(result.rows_affected() > 0)
    }

    async fn action_guild_id(&self, action_id: Uuid) -> Result<Option<String>, DomainError> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT guild_id FROM audit_logs \
             WHERE event_type LIKE 'mod_%' AND id = $1 \
             LIMIT 1",
        )
        .bind(action_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("fetch action guild_id"))?;
        Ok(row.map(|(g,)| g))
    }

    async fn count_recent_mod_actions(
        &self,
        guild_id: &str,
        moderator_id: &str,
        window_secs: i64,
    ) -> Result<i64, DomainError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_logs \
             WHERE guild_id = $1 AND actor_id = $2 AND event_type LIKE 'mod_%' \
               AND event_type NOT IN ('mod_unban', 'mod_unmute') \
               AND created_at > NOW() - ($3::double precision * INTERVAL '1 second')",
        )
        .bind(guild_id)
        .bind(moderator_id)
        .bind(window_secs as f64)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("count recent mod actions"))?;
        Ok(count)
    }

    async fn find_action_for_reversal(
        &self,
        action_id: Uuid,
    ) -> Result<
        Option<platform_core::sentinel::domain::entities::moderation::action::reversal::ActionReversalInfo>,
        DomainError,
    >{
        let row: Option<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
            "SELECT guild_id, target_id, target_name, event_type \
             FROM audit_logs \
             WHERE event_type LIKE 'mod_%' AND id = $1 \
             LIMIT 1",
        )
        .bind(action_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("fetch action"))?;

        Ok(
            row.map(|(guild_id, target_id_opt, target_name_opt, event_type)| {
                let action_type = ModerationAction::action_type_from_audit_event(&event_type)
                    .unwrap_or(&event_type)
                    .to_string();
                platform_core::sentinel::domain::entities::moderation::action::reversal::ActionReversalInfo {
                    guild_id,
                    target_id: target_id_opt.unwrap_or_default(),
                    target_name: target_name_opt.unwrap_or_default(),
                    action_type,
                }
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_action() -> ModerationAction {
        ModerationAction {
            id: Uuid::new_v4(),
            guild_id: "guild-1".into(),
            channel_id: "channel-1".into(),
            moderator_id: "moderator-1".into(),
            moderator_name: "Moderator".into(),
            target_id: "target-1".into(),
            target_name: "Target".into(),
            target_display_name: None,
            action_type: "warn".into(),
            reason: "raison".into(),
            gravity: Some(ModerationGravity::Medium),
            duration: Some(60),
            created_at: chrono::Utc::now(),
        }
    }

    #[sqlx::test(migrations = "../sentinel-api/migrations")]
    async fn moderation_action_round_trips_through_audit_logs(pool: PgPool) -> sqlx::Result<()> {
        let repo = PgModerationRepository::new(pool.clone());
        let action = sample_action();

        repo.save(&action).await.unwrap();

        let stored = repo.find_by_id(action.id).await.unwrap().unwrap();
        assert_eq!(stored.id, action.id);
        assert_eq!(stored.action_type, "warn");
        assert_eq!(stored.reason, "raison");
        assert_eq!(stored.duration, Some(60));
        assert_eq!(
            repo.count_recent_mod_actions("guild-1", "moderator-1", 3600)
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_logs WHERE id = $1")
                .bind(action.id)
                .fetch_one(&pool)
                .await?,
            1
        );
        Ok(())
    }
}
