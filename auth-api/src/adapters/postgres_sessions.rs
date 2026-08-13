//! Stockage Postgres des sessions et du journal des logins.
//!
//! Porté depuis
//! `sentinel-api/src/adapters/outbound/postgres/system/oauth_session_repository.rs`,
//! vers la base de l'identité.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use auth_core::domain::entities::session::{
    LoginTrace, NewOAuthSession, OAuthSession, SessionTokenUpdate, SuccessfulLogin,
};
use auth_core::domain::errors::DomainError;
use auth_core::ports::outbound::session_repository::SessionRepository;

pub struct PgSessionRepository {
    pool: PgPool,
}

impl PgSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn db_err(context: &'static str) -> impl Fn(sqlx::Error) -> DomainError {
    move |error| {
        // Le détail SQL reste dans les logs : il nomme des colonnes et des
        // contraintes, qui n'ont rien à faire dans une réponse HTTP.
        tracing::error!(%error, "{context}");
        DomainError::Internal(context.into())
    }
}

#[async_trait]
impl SessionRepository for PgSessionRepository {
    async fn create_session(&self, s: &NewOAuthSession) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO web_oauth_sessions \
             (id, discord_user_id, username, global_name, avatar, access_token, \
               refresh_token, access_expires_at, expires_at) \
              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(s.id)
        .bind(&s.discord_user_id)
        .bind(&s.username)
        .bind(&s.global_name)
        .bind(&s.avatar)
        .bind(&s.access_token)
        .bind(&s.refresh_token)
        .bind(s.access_expires_at)
        .bind(s.expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(db_err("creation de session impossible"))
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<OAuthSession>, DomainError> {
        let row = sqlx::query(
            "SELECT id, discord_user_id, username, global_name, avatar, access_token, \
                    refresh_token, access_expires_at, expires_at \
             FROM web_oauth_sessions WHERE id = $1 AND expires_at > now()",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err("lecture de session impossible"))?;

        Ok(row.map(|r| OAuthSession {
            id: r.get("id"),
            discord_user_id: r.get("discord_user_id"),
            username: r.get("username"),
            global_name: r.get("global_name"),
            avatar: r.get("avatar"),
            access_token: r.get("access_token"),
            refresh_token: r.get("refresh_token"),
            access_expires_at: r.get::<DateTime<Utc>, _>("access_expires_at"),
            expires_at: r.get::<DateTime<Utc>, _>("expires_at"),
        }))
    }

    async fn update_tokens(&self, u: &SessionTokenUpdate) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE web_oauth_sessions \
             SET access_token = $2, refresh_token = $3, access_expires_at = $4, \
                 last_used_at = now() \
             WHERE id = $1",
        )
        .bind(u.id)
        .bind(&u.access_token)
        .bind(&u.refresh_token)
        .bind(u.access_expires_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(db_err("mise a jour des jetons impossible"))
    }

    async fn touch(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE web_oauth_sessions SET last_used_at = now() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(db_err("touch de session impossible"))
    }

    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        sqlx::query("DELETE FROM web_oauth_sessions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(db_err("suppression de session impossible"))
    }

    async fn purge_expired(&self) -> Result<u64, DomainError> {
        sqlx::query("DELETE FROM web_oauth_sessions WHERE expires_at <= now()")
            .execute(&self.pool)
            .await
            .map(|result| result.rows_affected())
            .map_err(db_err("purge des sessions expirees impossible"))
    }

    async fn record_login(&self, t: &LoginTrace) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO successful_logins (discord_user_id, username, client_ip, user_agent) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(&t.discord_user_id)
        .bind(&t.username)
        .bind(&t.client_ip)
        .bind(&t.user_agent)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(db_err("trace de login impossible"))
    }

    async fn list_recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        let rows = sqlx::query(
            "SELECT discord_user_id, username, client_ip, user_agent, logged_at \
             FROM successful_logins ORDER BY logged_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err("lecture des logins impossible"))?;

        Ok(rows
            .into_iter()
            .map(|r| SuccessfulLogin {
                discord_user_id: r.get("discord_user_id"),
                username: r.get("username"),
                client_ip: r.get("client_ip"),
                user_agent: r.get("user_agent"),
                logged_at: r.get::<DateTime<Utc>, _>("logged_at"),
            })
            .collect())
    }

    async fn purge_logins_older_than(&self, days: i32) -> Result<u64, DomainError> {
        let result = sqlx::query(
            "DELETE FROM successful_logins \
             WHERE logged_at < now() - ($1 || ' days')::interval",
        )
        .bind(days.to_string())
        .execute(&self.pool)
        .await
        .map_err(db_err("purge des logins impossible"))?;
        Ok(result.rows_affected())
    }
}
