use crate::nexus::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use platform_core::nexus::domain::entities::game::player_session::PlayerSession;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::player_session_repository::PlayerSessionRepository;

pub struct PgPlayerSessionRepository {
    pool: PgPool,
}

impl PgPlayerSessionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct SessionRow {
    id: Uuid,
    server_id: Uuid,
    player_name: String,
    joined_at: DateTime<Utc>,
    left_at: Option<DateTime<Utc>>,
    duration_seconds: Option<i32>,
}

impl From<SessionRow> for PlayerSession {
    fn from(r: SessionRow) -> Self {
        Self {
            id: r.id,
            server_id: r.server_id,
            player_name: r.player_name,
            joined_at: r.joined_at,
            left_at: r.left_at,
            duration_seconds: r.duration_seconds,
        }
    }
}

#[async_trait]
impl PlayerSessionRepository for PgPlayerSessionRepository {
    async fn open(&self, server_id: Uuid, player_name: &str) -> Result<Uuid, DomainError> {
        let id: (Uuid,) = sqlx::query_as(
            "INSERT INTO game_player_sessions (server_id, player_name) \
             VALUES ($1, $2) RETURNING id",
        )
        .bind(server_id)
        .bind(player_name)
        .fetch_one(&self.pool)
        .await
        .map_err(pg_ctx("open session"))?;
        Ok(id.0)
    }

    async fn close(&self, server_id: Uuid, player_name: &str) -> Result<(), DomainError> {
        // Ferme la session active la plus ancienne pour ce joueur.
        sqlx::query(
            "UPDATE game_player_sessions SET left_at = NOW() \
             WHERE id = ( \
                 SELECT id FROM game_player_sessions \
                 WHERE server_id = $1 AND player_name = $2 AND left_at IS NULL \
                 ORDER BY joined_at ASC LIMIT 1 \
             )",
        )
        .bind(server_id)
        .bind(player_name)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("close session"))?;
        Ok(())
    }

    async fn list_active(&self, server_id: Uuid) -> Result<Vec<PlayerSession>, DomainError> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT id, server_id, player_name, joined_at, left_at, duration_seconds \
             FROM game_player_sessions \
             WHERE server_id = $1 AND left_at IS NULL",
        )
        .bind(server_id)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list_active sessions"))?;
        Ok(rows.into_iter().map(PlayerSession::from).collect())
    }

    async fn list_history(
        &self,
        server_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<PlayerSession>, DomainError> {
        let rows: Vec<SessionRow> = sqlx::query_as(
            "SELECT id, server_id, player_name, joined_at, left_at, duration_seconds \
             FROM game_player_sessions \
             WHERE server_id = $1 \
             ORDER BY joined_at DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(server_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list_history sessions"))?;
        Ok(rows.into_iter().map(PlayerSession::from).collect())
    }

    async fn close_all_active(&self, server_id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_player_sessions SET left_at = NOW() \
             WHERE server_id = $1 AND left_at IS NULL",
        )
        .bind(server_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("close_all_active"))?;
        Ok(())
    }
}
