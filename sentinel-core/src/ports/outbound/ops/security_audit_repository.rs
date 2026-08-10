//! Port outbound : journal d'audit, logins reussis et purge des logs securite.

use async_trait::async_trait;

use crate::domain::entities::ops::security_audit::{
    AuditLogEntry, AuditLogFilter, CleanupOptions, CleanupReport, SuccessfulLogin,
};
use crate::domain::errors::DomainError;

#[async_trait]
pub trait SecurityAuditRepository: Send + Sync {
    async fn list_audit_logs(
        &self,
        filter: AuditLogFilter,
    ) -> Result<Vec<AuditLogEntry>, DomainError>;

    async fn list_recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError>;

    /// Purge destructive des tables de logs selon les options. Best-effort
    /// par table (un echec sur une table n'annule pas les autres).
    async fn cleanup(&self, options: CleanupOptions) -> Result<CleanupReport, DomainError>;
}
