//! Adapter sortant Postgres des echeances d'acceptation du reglement
//! (`welcome_rules_pending`). Tout le SQL de ce domaine vit ici.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_err_ctx;
use platform_core::sentinel::domain::entities::community::rules_deadline::PendingRulesDeadline;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::rules_deadline_repository::RulesDeadlineRepository;

pub struct PgRulesDeadlineRepository {
    pool: PgPool,
}

impl PgRulesDeadlineRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type PendingRow = (String, String, DateTime<Utc>, Option<DateTime<Utc>>);

fn to_pending(row: PendingRow) -> PendingRulesDeadline {
    PendingRulesDeadline {
        guild_id: row.0,
        user_id: row.1,
        expires_at: row.2,
        reminded_at: row.3,
    }
}

#[async_trait]
impl RulesDeadlineRepository for PgRulesDeadlineRepository {
    async fn insert_if_absent(
        &self,
        guild_id: &str,
        user_id: &str,
        expires_at: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        // DO NOTHING, et non DO UPDATE : un membre qui repasse par l'accueil ne
        // doit pas voir son compte a rebours reparti de zero. Ce serait un
        // sursis illimite pour qui sait provoquer l'evenement.
        sqlx::query(
            "INSERT INTO welcome_rules_pending (guild_id, user_id, expires_at) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (guild_id, user_id) DO NOTHING",
        )
        .bind(guild_id)
        .bind(user_id)
        .bind(expires_at)
        .execute(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("insert rules deadline", e))?;
        Ok(())
    }

    async fn list_reminder_due(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingRulesDeadline>, DomainError> {
        // La fenetre de relance est calculee par le domaine, qui seul connait
        // `reminder_secs`. Ici on rend simplement ce qui n'a pas encore ete
        // relance et n'a pas expire ; l'appelant tranche.
        let rows: Vec<PendingRow> = sqlx::query_as(
            "SELECT guild_id, user_id, expires_at, reminded_at \
             FROM welcome_rules_pending \
             WHERE reminded_at IS NULL AND expires_at > NOW() \
             ORDER BY expires_at ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("list rules reminder due", e))?;
        Ok(rows.into_iter().map(to_pending).collect())
    }

    async fn claim_reminder(&self, guild_id: &str, user_id: &str) -> Result<bool, DomainError> {
        // La garde `reminded_at IS NULL` rend l'operation atomique : deux
        // instances qui reclament la meme relance, une seule l'obtient.
        let res = sqlx::query(
            "UPDATE welcome_rules_pending SET reminded_at = NOW() \
             WHERE guild_id = $1 AND user_id = $2 AND reminded_at IS NULL",
        )
        .bind(guild_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("claim rules reminder", e))?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_expired(&self, limit: i64) -> Result<Vec<PendingRulesDeadline>, DomainError> {
        let rows: Vec<PendingRow> = sqlx::query_as(
            "SELECT guild_id, user_id, expires_at, reminded_at \
             FROM welcome_rules_pending \
             WHERE expires_at <= NOW() \
             ORDER BY expires_at ASC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| pg_err_ctx("list rules expired", e))?;
        Ok(rows.into_iter().map(to_pending).collect())
    }

    async fn delete(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM welcome_rules_pending WHERE guild_id = $1 AND user_id = $2")
            .bind(guild_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| pg_err_ctx("delete rules deadline", e))?;
        Ok(())
    }
}
