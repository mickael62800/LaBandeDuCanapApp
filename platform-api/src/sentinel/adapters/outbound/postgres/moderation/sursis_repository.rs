//! Impl Postgres de `SursisRepository`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::sursis::{Sursis, SursisStatus};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::moderation::sursis_repository::{
    NewSursis, SursisRepository,
};

use super::super::pg_err_ctx;

const TBL: &str = "moderation_sursis";
fn pg_err(e: sqlx::Error) -> DomainError {
    pg_err_ctx(TBL, e)
}

pub struct PgSursisRepository {
    pool: PgPool,
}

impl PgSursisRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    guild_id: String,
    user_id: String,
    username: String,
    reason: String,
    saved_roles: serde_json::Value,
    channel_id: Option<String>,
    status: String,
    expires_at: DateTime<Utc>,
}

impl TryFrom<Row> for Sursis {
    type Error = DomainError;
    fn try_from(r: Row) -> Result<Self, DomainError> {
        let status = SursisStatus::from_str_lossy(&r.status).ok_or_else(|| {
            DomainError::Internal(format!("statut sursis inconnu : {}", r.status))
        })?;
        let saved_roles = r
            .saved_roles
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Ok(Self {
            id: r.id,
            guild_id: r.guild_id,
            user_id: r.user_id,
            username: r.username,
            reason: r.reason,
            saved_roles,
            channel_id: r.channel_id,
            status,
            expires_at: r.expires_at,
        })
    }
}

const COLS: &str = "id, guild_id, user_id, username, reason, saved_roles, channel_id, \
    status, expires_at";

#[async_trait]
impl SursisRepository for PgSursisRepository {
    async fn create(&self, new: NewSursis<'_>) -> Result<Sursis, DomainError> {
        let roles = serde_json::json!(new.saved_roles);
        let sql = format!(
            "INSERT INTO moderation_sursis \
             (guild_id, user_id, username, moderator_id, moderator_name, reason, saved_roles, channel_id, expires_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING {COLS}"
        );
        let row: Row = sqlx::query_as(&sql)
            .bind(new.guild_id)
            .bind(new.user_id)
            .bind(new.username)
            .bind(new.moderator_id)
            .bind(new.moderator_name)
            .bind(new.reason)
            .bind(roles)
            .bind(new.channel_id)
            .bind(new.expires_at)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match &e {
                sqlx::Error::Database(db) if db.is_unique_violation() => {
                    DomainError::Conflict("Ce membre est deja en sursis.".into())
                }
                _ => pg_err(e),
            })?;
        row.try_into()
    }

    async fn get(&self, id: Uuid) -> Result<Option<Sursis>, DomainError> {
        let sql = format!("SELECT {COLS} FROM moderation_sursis WHERE id = $1");
        let row: Option<Row> = sqlx::query_as(&sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(pg_err)?;
        row.map(TryInto::try_into).transpose()
    }

    async fn set_status(&self, id: Uuid, status: SursisStatus) -> Result<bool, DomainError> {
        // Garde d'etat : on ne transitionne QUE depuis 'en_sursis' (TOCTOU). Le
        // resultat (claim) permet a l'appelant de n'agir que s'il a gagne la
        // course -> pas de double ban / double pardon.
        let res = sqlx::query(
            "UPDATE moderation_sursis SET status = $2 WHERE id = $1 AND status = 'en_sursis'",
        )
        .bind(id)
        .bind(status.as_str())
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;
        Ok(res.rows_affected() == 1)
    }

    async fn list_due(&self, now: DateTime<Utc>) -> Result<Vec<Sursis>, DomainError> {
        let sql = format!(
            "SELECT {COLS} FROM moderation_sursis \
             WHERE status = 'en_sursis' AND expires_at <= $1"
        );
        let rows: Vec<Row> = sqlx::query_as(&sql)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(pg_err)?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}
