//! Adapter sortant Postgres du slowmode de securite manuel
//! (`security_slowmode_active`). Tout le SQL du domaine slowmode vit ici.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::slowmode_repository::SlowmodeRepository;

pub struct PgSlowmodeRepository {
    pool: PgPool,
}

impl PgSlowmodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SlowmodeRepository for PgSlowmodeRepository {
    async fn upsert(
        &self,
        guild_id: &str,
        previous_states: &serde_json::Value,
        expires_at: DateTime<Utc>,
        imposed_rate: i32,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO security_slowmode_active (guild_id, previous_states, expires_at, imposed_rate) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (guild_id) DO UPDATE SET \
                 previous_states = EXCLUDED.previous_states, \
                 expires_at = EXCLUDED.expires_at, \
                 imposed_rate = EXCLUDED.imposed_rate",
        )
        .bind(guild_id)
        .bind(previous_states)
        .bind(expires_at)
        .bind(imposed_rate)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn delete(&self, guild_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM security_slowmode_active WHERE guild_id = $1")
            .bind(guild_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }
}
