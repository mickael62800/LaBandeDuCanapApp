//! Adapter Postgres des delais Coussin Piégé (`nexus_coussin_cooldowns`).

use async_trait::async_trait;
use platform_core::nexus::{
    domain::errors::DomainError,
    ports::outbound::coussin_cooldown_repository::CoussinCooldownRepository,
};
use sqlx::PgPool;

use super::pg_err;

pub struct PgCoussinCooldownRepository {
    pool: PgPool,
}

impl PgCoussinCooldownRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CoussinCooldownRepository for PgCoussinCooldownRepository {
    async fn remaining_seconds(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
    ) -> Result<Option<i64>, DomainError> {
        let row: Option<(f64,)> = sqlx::query_as(
            "SELECT EXTRACT(EPOCH FROM (available_at - NOW()))
             FROM nexus_coussin_cooldowns
             WHERE guild_id = $1 AND user_id = $2 AND action = $3 AND available_at > NOW()",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(action)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(|(secondes,)| secondes.ceil().max(1.0) as i64))
    }

    async fn arm(
        &self,
        guild_id: &str,
        user_id: &str,
        action: &str,
        minutes: i64,
    ) -> Result<(), DomainError> {
        // Pas de delai reglé : rien a ecrire. Inserer une ligne deja expiree
        // remplirait la table sans jamais rien empecher.
        if minutes <= 0 {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO nexus_coussin_cooldowns (guild_id, user_id, action, available_at)
             VALUES ($1, $2, $3, NOW() + make_interval(mins => $4::int))
             ON CONFLICT (guild_id, user_id, action)
             DO UPDATE SET available_at = EXCLUDED.available_at",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(action)
        // Une semaine de plafond : au-dela, c'est une desactivation deguisee,
        // et mieux vaut alors decocher l'action.
        .bind(minutes.clamp(1, 10080) as i32)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
