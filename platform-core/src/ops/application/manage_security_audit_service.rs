//! Implementation du use case "audit & maintenance" securite. Pass-through
//! vers le repo (le SQL — y compris la purge multi-tables — est dans l'adapter).

use std::sync::Arc;

use async_trait::async_trait;

use crate::ops::domain::entities::security_audit::{
    AuditLogEntry, AuditLogFilter, CleanupOptions, CleanupReport, SuccessfulLogin,
};
use crate::ops::domain::errors::DomainError;
use crate::ops::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase;
use crate::ops::ports::outbound::security_audit_repository::SecurityAuditRepository;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::domain::entities::security_audit::CleanupTargetStatus;

    struct FakeSecurityAuditRepo;
    #[async_trait]
    impl SecurityAuditRepository for FakeSecurityAuditRepo {
        async fn list_audit_logs(
            &self,
            _filter: AuditLogFilter,
        ) -> Result<Vec<AuditLogEntry>, DomainError> {
            Ok(vec![])
        }

        async fn list_recent_logins(&self, _limit: i64) -> Result<Vec<SuccessfulLogin>, DomainError> {
            Ok(vec![])
        }

        async fn cleanup(&self, _options: CleanupOptions) -> Result<CleanupReport, DomainError> {
            Ok(CleanupReport {
                api_logs: CleanupTargetStatus::Skipped,
                audit_logs: CleanupTargetStatus::Skipped,
                server_events: CleanupTargetStatus::Skipped,
                successful_logins: CleanupTargetStatus::Skipped,
                manual_bans: CleanupTargetStatus::Skipped,
            })
        }
    }

    #[test]
    fn service_can_be_created() {
        let _service = ManageSecurityAuditService::new(Arc::new(FakeSecurityAuditRepo));
    }

    #[tokio::test]
    async fn audit_logs_delegates_to_repo() {
        let service = ManageSecurityAuditService::new(Arc::new(FakeSecurityAuditRepo));
        let filter = AuditLogFilter {
            guild_id: None,
            event_type_prefix: None,
            limit: 10,
        };
        let result = service.audit_logs(filter).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn recent_logins_delegates_to_repo() {
        let service = ManageSecurityAuditService::new(Arc::new(FakeSecurityAuditRepo));
        let result = service.recent_logins(10).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn cleanup_delegates_to_repo() {
        let service = ManageSecurityAuditService::new(Arc::new(FakeSecurityAuditRepo));
        let options = CleanupOptions {
            older_than_days: 30,
            include_api_logs: false,
            include_audit_logs: false,
            include_server_events: false,
            include_successful_logins: false,
            include_manual_bans: false,
        };
        let result = service.cleanup(options).await;
        assert!(result.is_ok());
    }
}
