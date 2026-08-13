use crate::sentinel::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use platform_core::sentinel::domain::entities::community::age_ban::{AgeBan, AgeBanStatus};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::community::age_ban_repository::AgeBanRepository;

#[derive(FromRow)]
struct AgeBanRow {
    id: Uuid,
    guild_id: String,
    user_id: String,
    declared_age: i32,
    banned_at: DateTime<Utc>,
    unban_at: DateTime<Utc>,
    status: String,
    lifted_at: Option<DateTime<Utc>>,
}

impl From<AgeBanRow> for AgeBan {
    fn from(r: AgeBanRow) -> Self {
        Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            declared_age: r.declared_age,
            banned_at: r.banned_at,
            unban_at: r.unban_at,
            status: AgeBanStatus::from_str(&r.status),
            lifted_at: r.lifted_at,
        }
    }
}

pub struct PgAgeBanRepository {
    pool: PgPool,
}

impl PgAgeBanRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AgeBanRepository for PgAgeBanRepository {
    async fn create(&self, ban: &AgeBan) -> Result<(), DomainError> {
        sqlx::query(
            r#"INSERT INTO age_verification_bans
                (id, guild_id, user_id, declared_age, banned_at, unban_at, status, lifted_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(ban.id)
        .bind(&ban.guild_id)
        .bind(&ban.user_id)
        .bind(ban.declared_age)
        .bind(ban.banned_at)
        .bind(ban.unban_at)
        .bind(ban.status.as_str())
        .bind(ban.lifted_at)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }

    async fn list_due(&self, limit: i64) -> Result<Vec<AgeBan>, DomainError> {
        let rows = sqlx::query_as::<_, AgeBanRow>(
            r#"SELECT id, guild_id, user_id, declared_age, banned_at, unban_at, status, lifted_at
               FROM age_verification_bans
               WHERE status = 'pending' AND unban_at <= NOW()
               ORDER BY unban_at ASC
               LIMIT $1"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(rows.into_iter().map(AgeBan::from).collect())
    }

    async fn mark_lifted(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE age_verification_bans SET status = 'lifted', lifted_at = NOW() \
             WHERE id = $1 AND status = 'pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(())
    }
}
