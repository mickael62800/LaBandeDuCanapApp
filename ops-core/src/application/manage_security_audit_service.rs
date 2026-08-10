//! Implementation du use case "audit & maintenance" securite. Pass-through
//! vers le repo (le SQL — y compris la purge multi-tables — est dans l'adapter).

use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::entities::security_audit::{
    AuditLogEntry, AuditLogFilter, CleanupOptions, CleanupReport, SuccessfulLogin,
};
use crate::domain::errors::DomainError;
use crate::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase;
use crate::ports::outbound::security_audit_repository::SecurityAuditRepository;

pub struct ManageSecurityAuditService {
    repo: Arc<dyn SecurityAuditRepository>,
}

impl ManageSecurityAuditService {
    pub fn new(repo: Arc<dyn SecurityAuditRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageSecurityAuditUseCase for ManageSecurityAuditService {
    async fn audit_logs(&self, filter: AuditLogFilter) -> Result<Vec<AuditLogEntry>, DomainError> {
        self.repo.list_audit_logs(filter).await
    }

    async fn recent_logins(&self, limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
        self.repo.list_recent_logins(limit).await
    }

    async fn cleanup(&self, options: CleanupOptions) -> Result<CleanupReport, DomainError> {
        self.repo.cleanup(options).await
    }
}
