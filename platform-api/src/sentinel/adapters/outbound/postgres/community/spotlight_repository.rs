use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::sentinel::adapters::outbound::postgres::pg_err;
use platform_core::sentinel::domain::entities::community::spotlight::{
    Spotlight, UpsertSpotlightCommand,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::spotlight_repository::SpotlightRepository;

const COLS: &str = "id, guild_id, user_id, username, avatar, period, reason, \
                    chosen_by, created_at";

pub struct PgSpotlightRepository {
    pool: PgPool,
}

impl PgSpotlightRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SpotlightRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    avatar: Option<String>,
    period: String,
    reason: String,
    chosen_by: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<SpotlightRow> for Spotlight {
    fn from(r: SpotlightRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            avatar: r.avatar,
            period: r.period,
            reason: r.reason,
            chosen_by: r.chosen_by,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl SpotlightRepository for PgSpotlightRepository {
    async fn find_by_period(
        &self,
        guild_id: &str,
        period: &str,
    ) -> Result<Option<Spotlight>, DomainError> {
        let row: Option<SpotlightRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM community_spotlight \
             WHERE guild_id = $1 AND period = $2"
        ))
        .bind(guild_id)
        .bind(period)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn find_latest(&self, guild_id: &str) -> Result<Option<Spotlight>, DomainError> {
        // Tri sur `period` et non sur `created_at` : si le staff rattrape un
        // mois oublie apres coup, c'est bien le mois le plus recent qu'il
        // faut mettre en avant, pas la derniere saisie.
        let row: Option<SpotlightRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM community_spotlight \
             WHERE guild_id = $1 ORDER BY period DESC LIMIT 1"
        ))
        .bind(guild_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.map(Into::into))
    }

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<Spotlight>, DomainError> {
        let rows: Vec<SpotlightRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM community_spotlight \
             WHERE guild_id = $1 ORDER BY period DESC LIMIT $2"
        ))
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert(&self, cmd: &UpsertSpotlightCommand) -> Result<Spotlight, DomainError> {
        // Un seul membre par periode : redesigner remplace, la contrainte
        // d'unicite rendant l'erreur impossible plutot que rattrapee.
        let row: SpotlightRow = sqlx::query_as(&format!(
            "INSERT INTO community_spotlight \
                 (guild_id, user_id, username, avatar, period, reason, chosen_by) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (guild_id, period) DO UPDATE SET \
                 user_id = EXCLUDED.user_id, \
                 username = EXCLUDED.username, \
                 avatar = EXCLUDED.avatar, \
                 reason = EXCLUDED.reason, \
                 chosen_by = EXCLUDED.chosen_by \
             RETURNING {COLS}"
        ))
        .bind(&cmd.guild_id)
        .bind(&cmd.user_id)
        .bind(&cmd.username)
        .bind(&cmd.avatar)
        .bind(&cmd.period)
        .bind(&cmd.reason)
        .bind(&cmd.chosen_by)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(row.into())
    }

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError> {
        let res = sqlx::query("DELETE FROM community_spotlight WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(res.rows_affected() > 0)
    }
}
