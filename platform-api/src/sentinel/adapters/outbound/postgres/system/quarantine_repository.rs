//! Adapter sortant Postgres des quarantaines de securite
//! (`security_quarantine_pending`). Tout le SQL du domaine quarantine vit ici.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::system::quarantine::ActiveQuarantine;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::quarantine_repository::QuarantineRepository;

pub struct PgQuarantineRepository {
    pool: PgPool,
}

impl PgQuarantineRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QuarantineRepository for PgQuarantineRepository {
    async fn upsert(
        &self,
        guild_id: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO security_quarantine_pending (guild_id, user_id, expires_at) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id, user_id) DO UPDATE SET expires_at = EXCLUDED.expires_at",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_active(&self) -> Result<Vec<ActiveQuarantine>, DomainError> {
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT guild_id, user_id FROM security_quarantine_pending WHERE expires_at > NOW()",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows
            .into_iter()
            .map(|(guild_id, user_id)| ActiveQuarantine { guild_id, user_id })
            .collect())
    }

    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM security_quarantine_pending WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }
}
