//! BatchedPgAuditLogRepository — wrap PgAuditLogRepository avec un BatchWriter<AuditLog>.

use crate::sentinel::adapters::outbound::postgres::audit::audit_log_repository::PgAuditLogRepository;
use async_trait::async_trait;
use platform_core::sentinel::domain::entities::audit::audit_log::AuditLog;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use platform_core::sentinel::ports::outbound::audit::audit_log_repository::AuditLogRepository;
use sqlx::PgPool;
use sqlx::QueryBuilder;

use super::batch_writer::BatchWriter;
use super::batch_writer::BatchWriterConfig;
pub struct BatchedPgAuditLogRepository {
    inner: PgAuditLogRepository,
    writer: BatchWriter<AuditLog>,
}

impl BatchedPgAuditLogRepository {
    pub fn new(pool: PgPool, config: BatchWriterConfig) -> Self {
        let flush_pool = pool.clone();
        let writer = BatchWriter::spawn("audit_logs", config, move |batch: Vec<AuditLog>| {
            let pool = flush_pool.clone();
            async move { flush_audit_logs(&pool, batch).await }
        });

        Self {
            inner: PgAuditLogRepository::new(pool),
            writer,
        }
    }
}

async fn flush_audit_logs(pool: &PgPool, batch: Vec<AuditLog>) -> Result<(), String> {
    if batch.is_empty() {
        return Ok(());
    }

    let mut qb = QueryBuilder::new(
        "INSERT INTO audit_logs (id, guild_id, event_type, actor_id, actor_name, target_id, target_name, channel_id, channel_name, details, created_at) ",
    );
    qb.push_values(batch.iter(), |mut b, log| {
        b.push_bind(log.id)
            .push_bind(log.guild_id.as_str())
            .push_bind(&log.event_type)
            .push_bind(&log.actor_id)
            .push_bind(&log.actor_name)
            .push_bind(&log.target_id)
            .push_bind(&log.target_name)
            .push_bind(&log.channel_id)
            .push_bind(&log.channel_name)
            .push_bind(&log.details)
            .push_bind(log.created_at);
    });

    qb.build()
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| format!("flush audit_logs ({} rows): {e}", batch.len()))
}

#[async_trait]
impl AuditLogRepository for BatchedPgAuditLogRepository {
    async fn save(&self, log: &AuditLog) -> Result<(), DomainError> {
        self.writer.try_send(log.clone());
        Ok(())
    }

    async fn find_all(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        self.inner.find_all(guild_id, filters).await
    }

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        self.inner.delete_older_than_days(guild_id, days).await
    }
}
