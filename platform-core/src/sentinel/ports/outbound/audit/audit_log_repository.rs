use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::audit_log::AuditLog;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;

#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn save(&self, log: &AuditLog) -> Result<(), DomainError>;
    async fn find_all(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError>;
    /// Total correspondant aux filtres (hors limit/offset). Default 0 pour
    /// ne pas casser les mocks de test existants.
    async fn count(
        &self,
        _guild_id: Option<&str>,
        _filters: &AuditLogFilters,
    ) -> Result<i64, DomainError> {
        Ok(0)
    }
    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;

    /// Timeline d'un salon vocal : events `VOICE_TIMELINE_EVENT_TYPES` du
    /// channel, ordre chronologique ASC. Default : vide (mocks de test).
    async fn list_voice_channel_events(
        &self,
        _channel_id: &str,
        _limit: i64,
    ) -> Result<Vec<AuditLog>, DomainError> {
        Ok(vec![])
    }
}
