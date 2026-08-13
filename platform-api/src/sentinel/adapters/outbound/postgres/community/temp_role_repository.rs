use async_trait::async_trait;
use sqlx::PgPool;

use super::super::pg_err;
use platform_core::sentinel::ports::outbound::community::temp_role_repository::TempRole;
use platform_core::sentinel::ports::outbound::community::temp_role_repository::TempRoleRepository;

pub struct PgTempRoleRepository {
    pool: PgPool,
}

impl PgTempRoleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TempRoleRepository for PgTempRoleRepository {
    async fn create(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
        expires_at: &str,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        sqlx::query(
            "INSERT INTO temp_roles (guild_id, user_id, role_id, expires_at) \
             VALUES ($1, $2, $3, $4::timestamptz) \
             ON CONFLICT (guild_id, user_id, role_id) DO UPDATE SET expires_at = $4::timestamptz",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(role_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_active(
        &self,
        guild_id: &str,
    ) -> Result<Vec<TempRole>, platform_core::sentinel::domain::errors::DomainError> {
        #[derive(sqlx::FromRow)]
        struct Row {
            id: uuid::Uuid,
            guild_id: String,
            user_id: String,
            role_id: String,
            expires_at: chrono::DateTime<chrono::Utc>,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let rows: Vec<Row> = sqlx::query_as(
            "SELECT id, guild_id, user_id, role_id, expires_at, created_at \
             FROM temp_roles WHERE guild_id = $1 AND expires_at > NOW() ORDER BY expires_at ASC",
        )
        .bind(guild_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows
            .into_iter()
            .map(|r| TempRole {
                id: r.id,
                guild_id: r.guild_id.into(),
                user_id: r.user_id.into(),
                role_id: r.role_id.into(),
                expires_at: r.expires_at,
                created_at: r.created_at,
            })
            .collect())
    }

    async fn delete(
        &self,
        guild_id: &str,
        user_id: &str,
        role_id: &str,
    ) -> Result<(), platform_core::sentinel::domain::errors::DomainError> {
        sqlx::query("DELETE FROM temp_roles WHERE guild_id = $1 AND user_id = $2 AND role_id = $3")
            .bind(guild_id)
            .bind(user_id)
            .bind(role_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }
}
