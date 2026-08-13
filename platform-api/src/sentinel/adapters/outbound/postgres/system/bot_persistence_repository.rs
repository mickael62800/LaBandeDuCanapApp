//! Adapter sortant Postgres pour la persistance des donnees fire-and-forget
//! des bots (`user_levels.streak_*`). Tout le SQL de ce domaine vit ici.

use async_trait::async_trait;
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::system::bot_persistence_repository::BotPersistenceRepository;

pub struct PgBotPersistenceRepository {
    pool: PgPool,
}

impl PgBotPersistenceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BotPersistenceRepository for PgBotPersistenceRepository {
    async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        streak_current: i32,
        streak_best: i32,
        streak_last_day: i32,
        streak_last_year: i32,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE user_levels SET streak_current = $1, streak_best = $2, \
             streak_last_day = $3, streak_last_year = $4, updated_at = NOW() \
             WHERE guild_id = $5 AND user_id = $6",
        )
        .bind(streak_current)
        .bind(streak_best)
        .bind(streak_last_day)
        .bind(streak_last_year)
        .bind(guild_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
