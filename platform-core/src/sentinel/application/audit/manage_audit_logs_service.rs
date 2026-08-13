use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::entities::audit::audit_log::AuditLog;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::sentinel::ports::outbound::audit::audit_log_repository::AuditLogRepository;

pub struct ManageAuditLogsService {
    repo: Arc<dyn AuditLogRepository>,
}

impl ManageAuditLogsService {
    pub fn new(repo: Arc<dyn AuditLogRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageAuditLogsUseCase for ManageAuditLogsService {
    async fn create(&self, cmd: CreateAuditLogCommand) -> Result<AuditLog, DomainError> {
        let log = AuditLog {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id,
            event_type: cmd.event_type,
            actor_id: cmd.actor_id,
            actor_name: cmd.actor_name,
            target_id: cmd.target_id,
            target_name: cmd.target_name,
            channel_id: cmd.channel_id,
            channel_name: cmd.channel_name,
            details: cmd.details,
            created_at: Utc::now(),
        };
        self.repo.save(&log).await?;
        Ok(log)
    }

    async fn list_voice_channel_events(
        &self,
        channel_id: &str,
        limit: i64,
    ) -> Result<Vec<AuditLog>, DomainError> {
        self.repo.list_voice_channel_events(channel_id, limit).await
    }

    async fn list(
        &self,
        guild_id: Option<&str>,
        filters: AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        self.repo.find_all(guild_id, &filters).await
    }

    async fn count(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<i64, DomainError> {
        self.repo.count(guild_id, filters).await
    }

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError> {
        self.repo.delete_older_than_days(guild_id, days).await
    }
}

#[cfg(test)]
#[path = "tests/manage_audit_logs.rs"]
mod tests;
