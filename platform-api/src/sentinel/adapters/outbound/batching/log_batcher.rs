//! BatchedPgLogRepository — wrap PgLogRepository avec un BatchWriter<LogEntry>.
//!
//! Les `save()` enqueue en memoire et retournent `Ok(())` immediatement (at-most-once).
//! Les autres methodes delegent au repository direct.

use crate::sentinel::adapters::outbound::postgres::system::log_repository::PgLogRepository;
use async_trait::async_trait;
use platform_core::ops::domain::entities::log_entry::LogEntry;
use platform_core::ops::ports::outbound::log_repository::LogRepository;
use platform_core::sentinel::domain::errors::DomainError;
use sqlx::PgPool;
use sqlx::QueryBuilder;

use super::batch_writer::BatchWriter;
use super::batch_writer::BatchWriterConfig;
pub struct BatchedPgLogRepository {
    inner: PgLogRepository,
    writer: BatchWriter<LogEntry>,
}

impl BatchedPgLogRepository {
    /// Construit le repo batched et spawn le flusher.
    pub fn new(pool: PgPool, config: BatchWriterConfig) -> Self {
        let flush_pool = pool.clone();
        let writer = BatchWriter::spawn("logs", config, move |batch: Vec<LogEntry>| {
            let pool = flush_pool.clone();
            async move { flush_logs(&pool, batch).await }
        });

        Self {
            inner: PgLogRepository::new(pool),
            writer,
        }
    }
}

async fn flush_logs(pool: &PgPool, batch: Vec<LogEntry>) -> Result<(), String> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::new(
        "INSERT INTO logs (id, timestamp, level, bot, server, message, category, details) ",
    );
    qb.push_values(batch.iter(), |mut b, entry| {
        b.push_bind(entry.id)
            .push_bind(entry.timestamp)
            .push_bind(&entry.level)
            .push_bind(&entry.bot)
            .push_bind(&entry.server)
            .push_bind(&entry.message)
            .push_bind(&entry.category)
            .push_bind(&entry.details);
    });

    qb.build()
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| format!("flush logs ({} rows): {e}", batch.len()))
}

#[async_trait]
impl LogRepository for BatchedPgLogRepository {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError> {
        // Clone minimal pour l'enqueue — retour immediat, fire-and-forget.
        if !self.writer.try_send(entry.clone()) {
            // Queue pleine : on laisse passer silencieusement (deja warn dans BatchWriter).
            // Pas d'erreur remontee au caller pour ne pas bloquer le request path.
        }
        Ok(())
    }

    async fn find_all(&self, limit: i64) -> Result<Vec<LogEntry>, DomainError> {
        self.inner.find_all(limit).await
    }

    async fn find_filtered(
        &self,
        category: Option<&str>,
        level: Option<&str>,
        guild_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<LogEntry>, DomainError> {
        self.inner
            .find_filtered(category, level, guild_id, limit)
            .await
    }

    async fn delete_by_category(&self, category: &str) -> Result<u64, DomainError> {
        self.inner.delete_by_category(category).await
    }

    async fn delete_older_than_days(&self, days: i32) -> Result<u64, DomainError> {
        self.inner.delete_older_than_days(days).await
    }
}
