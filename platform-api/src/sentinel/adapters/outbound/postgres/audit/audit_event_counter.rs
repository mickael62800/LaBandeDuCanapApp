use async_trait::async_trait;
use sqlx::PgPool;

use crate::sentinel::adapters::outbound::postgres::pg_ctx;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::outbound::audit::audit_event_counter::AuditEventCounter;

/// Comptage postgres des events d'audit par `event_type` sur une fenetre en
/// jours. Alimente le use case `GetWeeklyReport` (agregation server-side).
pub struct PgAuditEventCounter {
    pool: PgPool,
}

impl PgAuditEventCounter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct EventCountRow {
    event_type: String,
    count: i64,
}

#[async_trait]
impl AuditEventCounter for PgAuditEventCounter {
    async fn count_by_event_type(
        &self,
        guild_id: &str,
        days: u32,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        let rows: Vec<EventCountRow> = sqlx::query_as(
            r#"SELECT event_type, COUNT(*) AS count
               FROM audit_logs
               WHERE guild_id = $1
                 AND created_at > NOW() - make_interval(days => $2)
               GROUP BY event_type"#,
        )
        .bind(guild_id)
        .bind(days as i32)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_ctx("count_audit_events_by_type"))?;

        Ok(rows
            .into_iter()
            .map(|r| (r.event_type, r.count.max(0) as u64))
            .collect())
    }
}
