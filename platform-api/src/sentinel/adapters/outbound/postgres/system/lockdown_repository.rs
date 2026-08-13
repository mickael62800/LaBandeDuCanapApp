//! Adapter sortant Postgres du lockdown de securite
//! (`security_lockdown_active`). Tout le SQL du domaine lockdown vit ici.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::lockdown_repository::LockdownRepository;

pub struct PgLockdownRepository {
    pool: PgPool,
}

impl PgLockdownRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LockdownRepository for PgLockdownRepository {
    async fn upsert(
        &self,
        guild_id: &str,
        saved_states: &serde_json::Value,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO security_lockdown_active (guild_id, saved_states, expires_at) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id) DO UPDATE SET \
                 saved_states = EXCLUDED.saved_states, \
                 expires_at = EXCLUDED.expires_at",
        )
        .bind(guild_id)
        .bind(saved_states)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete(&self, guild_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM security_lockdown_active WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }
}
