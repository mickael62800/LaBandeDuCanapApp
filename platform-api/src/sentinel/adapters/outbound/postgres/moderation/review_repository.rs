use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err;
use platform_core::sentinel::ports::outbound::moderation::review_repository::ReviewEntry;
use platform_core::sentinel::ports::outbound::moderation::review_repository::ReviewRepository;

pub struct PgReviewRepository {
    pool: PgPool,
}

impl PgReviewRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReviewRepository for PgReviewRepository {
    async fn add(
        &self,
        action_id: uuid::Uuid,
        guild_id: &str,
        added_by: &str,
        added_by_name: &str,
        reason: Option<&str>,
    ) -> Result<ReviewEntry, platform_core::sentinel::domain::errors::DomainError> {
        let row: (uuid::Uuid, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
            "INSERT INTO review_queue \
                 (action_id, action_created_at, guild_id, added_by, added_by_name, reason) \
             SELECT a.id, a.created_at, $2, $3, $4, $5 \
             FROM audit_logs a \
             WHERE a.id = $1 AND a.event_type LIKE 'mod_%' \
             ORDER BY a.created_at DESC LIMIT 1 \
             RETURNING id, added_at",
        )
        .bind(action_id)
        .bind(guild_id)
        .bind(added_by)
        .bind(added_by_name)
        .bind(reason)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(ReviewEntry {
            id: row.0,
            action_id,
            guild_id: guild_id.into(),
            added_by: added_by.into(),
            added_by_name: added_by_name.into(),
            reason: reason.map(|s| s.into()),
            status: "pending".into(),
            reviewer_id: None,
            reviewer_name: None,
            reviewer_notes: None,
            added_at: row.1,
            resolved_at: None,
            action_type: None,
            target_name: None,
            action_reason: None,
        })
    }

    async fn list_pending(
        &self,
        guild_id: &str,
    ) -> Result<Vec<ReviewEntry>, platform_core::sentinel::domain::errors::DomainError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: uuid::Uuid,
            action_id: uuid::Uuid,
            added_by: String,
            added_by_name: String,
            reason: Option<String>,
            added_at: chrono::DateTime<chrono::Utc>,
            action_type: String,
            target_name: String,
            action_reason: String,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT r.id, r.action_id, r.added_by, r.added_by_name, r.reason, r.added_at, \
                    regexp_replace(a.event_type, '^mod_', '') AS action_type, \
                    COALESCE(a.target_name, '') AS target_name, \
                    COALESCE(a.details->>'reason', '') AS action_reason \
             FROM review_queue r \
             INNER JOIN audit_logs a \
                ON a.id = r.action_id AND a.created_at = r.action_created_at \
             WHERE r.guild_id = $1 AND r.status = 'pending' \
             ORDER BY r.added_at ASC LIMIT 50",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| ReviewEntry {
                id: r.id,
                action_id: r.action_id,
                guild_id: guild_id.into(),
                added_by: r.added_by,
                added_by_name: r.added_by_name,
                reason: r.reason,
                status: "pending".into(),
                reviewer_id: None,
                reviewer_name: None,
                reviewer_notes: None,
                added_at: r.added_at,
                resolved_at: None,
                action_type: Some(r.action_type),
                target_name: Some(r.target_name),
                action_reason: Some(r.action_reason),
            })
            .collect())
    }

    async fn resolve(
        &self,
        review_id: uuid::Uuid,
        reviewer_id: &str,
        reviewer_name: &str,
        notes: Option<&str>,
        status: &str,
    ) -> Result<bool, platform_core::sentinel::domain::errors::DomainError> {
        let res = sqlx::query(
            "UPDATE review_queue SET status = $1, reviewer_id = $2, reviewer_name = $3, \
             reviewer_notes = $4, resolved_at = NOW() WHERE id = $5",
        )
        .bind(status)
        .bind(reviewer_id)
        .bind(reviewer_name)
        .bind(notes)
        .bind(review_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }

    async fn get_guild_id(
        &self,
        review_id: uuid::Uuid,
    ) -> Result<Option<String>, platform_core::sentinel::domain::errors::DomainError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT guild_id FROM review_queue WHERE id = $1")
                .bind(review_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.map(|r| r.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations/sentinel")]
    async fn review_is_linked_to_partitioned_audit_action(pool: PgPool) -> sqlx::Result<()> {
        let action_id = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO audit_logs \
                 (id, guild_id, event_type, actor_id, target_id, target_name, details) \
             VALUES ($1, 'guild-1', 'mod_warn', 'moderator-1', 'target-1', 'Target', \
                     '{\"reason\":\"spam\"}'::jsonb)",
        )
        .bind(action_id)
        .execute(&pool)
        .await?;

        let repo = PgReviewRepository::new(pool);
        repo.add(action_id, "guild-1", "admin-1", "Admin", None)
            .await
            .unwrap();
        let pending = repo.list_pending("guild-1").await.unwrap();

        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].action_id, action_id);
        assert_eq!(pending[0].action_type.as_deref(), Some("warn"));
        assert_eq!(pending[0].action_reason.as_deref(), Some("spam"));
        Ok(())
    }
}
