//! Use case : journal d'audit admin, derniers logins, purge des logs securite.

use async_trait::async_trait;

use crate::ops::domain::entities::security_audit::{
    AuditLogEntry, AuditLogFilter, CleanupOptions, CleanupReport, SuccessfulLogin,
};
use crate::ops::domain::errors::DomainError;

#[async_trait]
pub trait ManageSecurityAuditUseCase: Send + Sync {
    async fn audit_logs(&self, filter: AuditLogFilter) -> Result<Vec<AuditLogEntry>, DomainError>;

    async fn recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError>;

    async fn cleanup(&self, options: CleanupOptions) -> Result<CleanupReport, DomainError>;
}
