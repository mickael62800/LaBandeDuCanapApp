use crate::nexus::adapters::outbound::postgres::pg_ctx;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use platform_core::nexus::domain::entities::game::audit::{GameAuditAction, GameAuditEntry};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::game_audit_repository::GameAuditRepository;

pub struct PgGameAuditRepository {
    pool: PgPool,
}

impl PgGameAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct AuditRow {
    id: Uuid,
    server_id: Option<Uuid>,
    guild_id: String,
    actor_user_id: Option<String>,
    action: String,
    details: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl From<AuditRow> for GameAuditEntry {
    fn from(r: AuditRow) -> Self {
        Self {
            id: r.id,
            server_id: r.server_id,
            guild_id: r.guild_id,
            actor_user_id: r.actor_user_id,
            action: r.action,
            details: r.details,
            created_at: r.created_at,
        }
    }
}

#[async_trait]
impl GameAuditRepository for PgGameAuditRepository {
    async fn log(
        &self,
        guild_id: &str,
        server_id: Option<Uuid>,
        actor_user_id: Option<&str>,
        action: GameAuditAction,
        details: serde_json::Value,
    ) -> Result<(), DomainError> {
        sqlx::query(
            "INSERT INTO game_audit_log (server_id, guild_id, actor_user_id, action, details) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(server_id)
        .bind(guild_id)
        .bind(actor_user_id)
        .bind(action.as_str())
        .bind(details)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("log audit"))?;
        Ok(())
    }

    async fn list_for_server(
        &self,
        server_id: Uuid,
        limit: i64,
    ) -> Result<Vec<GameAuditEntry>, DomainError> {
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT id, server_id, guild_id, actor_user_id, action, details, created_at \
             FROM game_audit_log \
             WHERE server_id = $1 \
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(server_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list audit for server"))?;
        Ok(rows.into_iter().map(GameAuditEntry::from).collect())
    }

    async fn list_for_guild(
        &self,
        guild_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<GameAuditEntry>, DomainError> {
        let rows: Vec<AuditRow> = sqlx::query_as(
            "SELECT id, server_id, guild_id, actor_user_id, action, details, created_at \
             FROM game_audit_log \
             WHERE guild_id = $1 \
             ORDER BY created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(guild_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list audit for guild"))?;
        Ok(rows.into_iter().map(GameAuditEntry::from).collect())
    }
}
