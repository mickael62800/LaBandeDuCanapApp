use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_err;
use platform_core::sentinel::ports::outbound::moderation::pending_action_repository::PendingAction;
use platform_core::sentinel::ports::outbound::moderation::pending_action_repository::PendingActionRepository;

pub struct PgPendingActionRepository {
    pool: PgPool,
}

impl PgPendingActionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PendingActionRepository for PgPendingActionRepository {
    async fn create(
        &self,
        guild_id: &str,
        moderator_id: &str,
        moderator_name: &str,
        target_id: &str,
        target_name: &str,
        action_type: &str,
        reason: &str,
        gravity: Option<&str>,
        duration: Option<i64>,
    ) -> Result<Uuid, platform_core::sentinel::domain::errors::DomainError> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO pending_mod_actions \
             (guild_id, moderator_id, moderator_name, target_id, target_name, action_type, reason, gravity, duration) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
        )
        .bind(guild_id).bind(moderator_id).bind(moderator_name)
        .bind(target_id).bind(target_name).bind(action_type)
        .bind(reason).bind(gravity).bind(duration)
        .fetch_one(&self.pool).await.map_err(pg_err)?;
        Ok(id)
    }

    async fn list_pending(
        &self,
        guild_id: &str,
    ) -> Result<Vec<PendingAction>, platform_core::sentinel::domain::errors::DomainError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: Uuid,
            guild_id: String,
            moderator_id: String,
            moderator_name: String,
            target_id: String,
            target_name: String,
            action_type: String,
            reason: String,
            gravity: Option<String>,
            duration: Option<i64>,
            status: String,
            reviewed_by: Option<String>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
        }
        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, guild_id, moderator_id, moderator_name, target_id, target_name, \
             action_type, reason, gravity, duration, status, reviewed_by, created_at, updated_at \
             FROM pending_mod_actions WHERE guild_id = $1 AND status = 'pending' \
             ORDER BY created_at DESC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|r| PendingAction {
                id: r.id,
                guild_id: r.guild_id.into(),
                moderator_id: r.moderator_id,
                moderator_name: r.moderator_name,
                target_id: r.target_id,
                target_name: r.target_name,
                action_type: r.action_type,
                reason: r.reason,
                gravity: r.gravity,
                duration: r.duration,
                status: r.status,
                reviewed_by: r.reviewed_by,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
            .collect())
    }

    async fn get_guild_id(
        &self,
        id: Uuid,
    ) -> Result<Option<String>, platform_core::sentinel::domain::errors::DomainError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT guild_id FROM pending_mod_actions WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(row.map(|r| r.0))
    }

    async fn resolve(
        &self,
        id: Uuid,
        status: &str,
        reviewed_by: &str,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        // Garde d'etat : on ne resout qu'un pending encore 'pending' -> une
        // double resolution (approve puis reject concurrent) n'ecrase plus.
        sqlx::query(
            "UPDATE pending_mod_actions SET status = $1, reviewed_by = $2, updated_at = NOW() \
             WHERE id = $3 AND status = 'pending'",
        )
        .bind(status)
        .bind(reviewed_by)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
