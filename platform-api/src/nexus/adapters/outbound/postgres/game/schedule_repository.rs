use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_ctx;
use platform_core::nexus::domain::entities::game::schedule::TimeRange;
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::schedule_repository::{
    GameScheduleRepository, StoredSchedule,
};

pub struct PgGameScheduleRepository {
    pool: PgPool,
}

impl PgGameScheduleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type ScheduleRow = (
    Uuid,
    bool,
    String,
    serde_json::Value,
    i32,
    Option<chrono::DateTime<chrono::Utc>>,
);

const COLS: &str = "server_id, enabled, timezone, ranges, warn_minutes, last_warned_at";

fn to_schedule(row: ScheduleRow) -> StoredSchedule {
    // Des plages illisibles valent une absence de plages : le serveur reste
    // tel quel plutot que d'etre eteint sur un format qu'on ne comprend pas.
    let ranges: Vec<TimeRange> = serde_json::from_value(row.3).unwrap_or_default();
    StoredSchedule {
        server_id: row.0,
        enabled: row.1,
        timezone: row.2,
        ranges,
        warn_minutes: row.4.clamp(0, 120) as u16,
        last_warned_at: row.5,
    }
}

#[async_trait]
impl GameScheduleRepository for PgGameScheduleRepository {
    async fn find(&self, server_id: Uuid) -> Result<Option<StoredSchedule>, DomainError> {
        let row: Option<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM game_server_schedules WHERE server_id = $1"
        ))
        .bind(server_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(pg_ctx("find schedule"))?;
        Ok(row.map(to_schedule))
    }

    async fn list_enabled(&self) -> Result<Vec<StoredSchedule>, DomainError> {
        let rows: Vec<ScheduleRow> = sqlx::query_as(&format!(
            "SELECT {COLS} FROM game_server_schedules WHERE enabled = true"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("list enabled schedules"))?;
        Ok(rows.into_iter().map(to_schedule).collect())
    }

    async fn upsert(
        &self,
        server_id: Uuid,
        enabled: bool,
        timezone: &str,
        ranges: &[TimeRange],
        warn_minutes: u16,
        actor: Option<&str>,
    ) -> Result<(), DomainError> {
        let ranges_json = serde_json::to_value(ranges)
            .map_err(|e| DomainError::Internal(format!("plages illisibles: {e}")))?;
        sqlx::query(
            "INSERT INTO game_server_schedules \
                 (server_id, enabled, timezone, ranges, warn_minutes, updated_by) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             ON CONFLICT (server_id) DO UPDATE SET \
                 enabled = EXCLUDED.enabled, \
                 timezone = EXCLUDED.timezone, \
                 ranges = EXCLUDED.ranges, \
                 warn_minutes = EXCLUDED.warn_minutes, \
                 updated_at = NOW(), \
                 updated_by = EXCLUDED.updated_by",
        )
        .bind(server_id)
        .bind(enabled)
        .bind(timezone)
        .bind(ranges_json)
        .bind(warn_minutes as i32)
        .bind(actor)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("upsert schedule"))?;
        Ok(())
    }

    async fn mark_warned(&self, server_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE game_server_schedules SET last_warned_at = NOW() WHERE server_id = $1")
            .bind(server_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("mark warned"))?;
        Ok(())
    }

    async fn clear_warning(&self, server_id: Uuid) -> Result<(), DomainError> {
        sqlx::query("UPDATE game_server_schedules SET last_warned_at = NULL WHERE server_id = $1")
            .bind(server_id)
            .execute(&self.pool)
            .await
            .map_err(pg_ctx("clear warning"))?;
        Ok(())
    }
}
