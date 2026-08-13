//! Impl Postgres de `AdaptiveSlowmodeRepository`.

use async_trait::async_trait;
use sqlx::PgPool;

use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::adaptive_slowmode_repository::AdaptiveSlowmodeRepository;

use super::super::pg_err_ctx;

const TBL: &str = "automod_adaptive_slowmode";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgAdaptiveSlowmodeRepository {
    pool: PgPool,
}

impl PgAdaptiveSlowmodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AdaptiveSlowmodeRepository for PgAdaptiveSlowmodeRepository {
    async fn mark(&self, guild_id: &str, channel_id: &str) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO automod_adaptive_slowmode (channel_id, guild_id) VALUES ($1, $2) \
             ON CONFLICT (channel_id) DO NOTHING",
        )
        .bind(channel_id)
        .bind(guild_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn unmark(&self, channel_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM automod_adaptive_slowmode WHERE channel_id = $1")
            .bind(channel_id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(())
    }

    async fn list_all(&self) -> Result<Vec<(String, String)>, DomainError> {
        let rows: Vec<(String, String)> =
            sqlx::query_as("SELECT guild_id, channel_id FROM automod_adaptive_slowmode")
                .fetch_all(&self.pool)
                .await
                .map_err(pg_err)?;
        Ok(rows)
    }
}
