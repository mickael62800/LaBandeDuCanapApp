use crate::adapters::outbound::postgres::pg_ctx;
use crate::adapters::outbound::postgres::pg_err;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use ops_core::domain::entities::log_entry::LogEntry;
use ops_core::ports::outbound::log_repository::LogRepository;
use sentinel_core::domain::errors::DomainError;

pub struct PgLogRepository {
    pool: PgPool,
}

impl PgLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct LogRow {
    id: Uuid,
    timestamp: chrono::DateTime<chrono::Utc>,
    level: String,
    bot: String,
    server: String,
    message: String,
    category: String,
    details: serde_json::Value,
}

impl From<LogRow> for LogEntry {
    fn from(row: LogRow) -> Self {
        Self {
            id: row.id,
            timestamp: row.timestamp,
            level: row.level,
            bot: row.bot,
            server: row.server,
            message: row.message,
            category: row.category,
            details: row.details,
        }
    }
}

#[async_trait]
impl LogRepository for PgLogRepository {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError> {
        sqlx::query(
            r#"
            INSERT INTO logs (id, timestamp, level, bot, server, message, category, details)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(entry.id)
        .bind(entry.timestamp)
        .bind(&entry.level)
        .bind(&entry.bot)
        .bind(&entry.server)
        .bind(&entry.message)
        .bind(&entry.category)
        .bind(&entry.details)
        .execute(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(())
    }

    async fn find_all(&self, limit: i64) -> Result<Vec<LogEntry>, DomainError> {
        let rows = sqlx::query_as::<_, LogRow>(
            "SELECT id, timestamp, level, bot, server, message, category, details FROM logs ORDER BY timestamp DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(LogEntry::from).collect())
    }

    async fn find_filtered(
        &self,
        category: Option<&str>,
        level: Option<&str>,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LogEntry>, DomainError> {
        // Filtre dynamique : on construit la WHERE clause en fonction des
        // filtres optionnels (category / level / guild). Le filtre `guild_id`
        // est une clause SQL (`server = $3`) — plus de filtrage post-fetch en
        // Rust. Permet a la page Logs systeme de charger 200 lignes PAR colonne
        // (bot, worker, api, websocket) au lieu de partager un seul pool de 200.
        let rows = sqlx::query_as::<_, LogRow>(
            "SELECT id, timestamp, level, bot, server, message, category, details \
             FROM logs \
             WHERE ($1::text IS NULL OR category = $1) \
               AND ($2::text IS NULL OR level = $2) \
               AND ($3::text IS NULL OR server = $3) \
             ORDER BY timestamp DESC LIMIT $4",
        )
        .bind(category)
        .bind(level)
        .bind(guild_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(pg_err)?;

        Ok(rows.into_iter().map(LogEntry::from).collect())
    }

    async fn delete_by_category(&self, category: &str) -> Result<u64, DomainError> {
        let result = sqlx::query("DELETE FROM logs WHERE category = $1")
            .bind(category)
            .execute(&self.pool)
            .await
            .map_err(pg_err)?;
        Ok(result.rows_affected())
    }

    async fn delete_older_than_days(&self, days: i32) -> Result<u64, DomainError> {
        let result =
            sqlx::query("DELETE FROM logs WHERE timestamp < NOW() - make_interval(days => $1)")
                .bind(days)
                .execute(&self.pool)
                .await
                .map_err(pg_ctx("delete_logs_older"))?;
        Ok(result.rows_affected())
    }
}
