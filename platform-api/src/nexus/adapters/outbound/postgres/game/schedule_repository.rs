use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::pg_ctx;
use platform_core::nexus::domain::entities::game::schedule::{ScheduleMode, TimeRange};
use platform_core::nexus::domain::errors::DomainError;
use platform_core::nexus::ports::outbound::game::schedule_repository::{
    GameScheduleRepository, ScheduleSettings, StoredSchedule,
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
    String,
    Option<i32>,
    i32,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<chrono::DateTime<chrono::Utc>>,
);

const COLS: &str = "server_id, enabled, timezone, ranges, warn_minutes, last_warned_at, \
                    mode, restart_interval_hours, restart_anchor_minute, last_restart_at, \
                    last_final_warned_at";

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
        mode: ScheduleMode::from_str(&row.6),
        // La contrainte CHECK borne deja la valeur ; le clamp protege une base
        // dont la contrainte aurait ete levee a la main.
        restart_interval_hours: row.7.map(|h| h.clamp(1, 24) as u8),
        restart_anchor_minute: row.8.clamp(0, 59) as u8,
        last_restart_at: row.9,
        last_final_warned_at: row.10,
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
        settings: &ScheduleSettings,
        actor: Option<&str>,
    ) -> Result<(), DomainError> {
        let ranges_json = serde_json::to_value(&settings.ranges)
            .map_err(|e| DomainError::Internal(format!("plages illisibles: {e}")))?;
        // Changer de mode remet les marqueurs a zero : une annonce faite sous
        // l'ancien regime ne doit pas museler la premiere du nouveau.
        sqlx::query(
            "INSERT INTO game_server_schedules \
                 (server_id, enabled, timezone, ranges, warn_minutes, updated_by, \
                  mode, restart_interval_hours, restart_anchor_minute) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (server_id) DO UPDATE SET \
                 enabled = EXCLUDED.enabled, \
                 timezone = EXCLUDED.timezone, \
                 ranges = EXCLUDED.ranges, \
                 warn_minutes = EXCLUDED.warn_minutes, \
                 mode = EXCLUDED.mode, \
                 restart_interval_hours = EXCLUDED.restart_interval_hours, \
                 restart_anchor_minute = EXCLUDED.restart_anchor_minute, \
                 last_warned_at = CASE WHEN game_server_schedules.mode = EXCLUDED.mode \
                     THEN game_server_schedules.last_warned_at ELSE NULL END, \
                 last_final_warned_at = CASE WHEN game_server_schedules.mode = EXCLUDED.mode \
                     THEN game_server_schedules.last_final_warned_at ELSE NULL END, \
                 updated_at = NOW(), \
                 updated_by = EXCLUDED.updated_by",
        )
        .bind(server_id)
        .bind(settings.enabled)
        .bind(&settings.timezone)
        .bind(ranges_json)
        .bind(settings.warn_minutes as i32)
        .bind(actor)
        .bind(settings.mode.as_str())
        .bind(settings.restart_interval_hours.map(i32::from))
        .bind(i32::from(settings.restart_anchor_minute))
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

    async fn mark_final_warned(&self, server_id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_server_schedules SET last_final_warned_at = NOW() WHERE server_id = $1",
        )
        .bind(server_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("mark final warned"))?;
        Ok(())
    }

    async fn mark_restarted(&self, server_id: Uuid) -> Result<(), DomainError> {
        // Les deux marqueurs tombent avec le redemarrage : le creneau suivant
        // doit etre annonce comme le premier l'a ete.
        sqlx::query(
            "UPDATE game_server_schedules \
             SET last_restart_at = NOW(), last_warned_at = NULL, last_final_warned_at = NULL \
             WHERE server_id = $1",
        )
        .bind(server_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("mark restarted"))?;
        Ok(())
    }

    async fn clear_warning(&self, server_id: Uuid) -> Result<(), DomainError> {
        sqlx::query(
            "UPDATE game_server_schedules \
             SET last_warned_at = NULL, last_final_warned_at = NULL WHERE server_id = $1",
        )
        .bind(server_id)
        .execute(&self.pool)
        .await
        .map_err(pg_ctx("clear warning"))?;
        Ok(())
    }
}
