use async_trait::async_trait;
use std::sync::Arc;
use std::sync::Mutex;

use crate::sentinel::application::audit::manage_audit_logs_service::ManageAuditLogsService;
use crate::sentinel::domain::entities::audit::audit_log::AuditLog;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use crate::sentinel::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use crate::sentinel::ports::outbound::audit::audit_log_repository::AuditLogRepository;

#[derive(Default)]
struct MockRepo {
    saved: Mutex<Vec<AuditLog>>,
    deletions: Mutex<Vec<(String, i32)>>,
    find_calls: Mutex<Vec<(Option<String>, i64, i64)>>, // guild, limit, offset
}

#[async_trait]
impl AuditLogRepository for MockRepo {
    async fn save(&self, log: &AuditLog) -> Result<(), DomainError> {
        self.saved.lock().unwrap().push(log.clone());
        Ok(())
    }
    async fn find_all(
        &self,
        g: Option<&str>,
        f: &AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError> {
        self.find_calls
            .lock()
            .unwrap()
            .push((g.map(String::from), f.limit, f.offset));
        Ok(self.saved.lock().unwrap().clone())
    }
    async fn delete_older_than_days(&self, g: &str, d: i32) -> Result<u64, DomainError> {
        self.deletions.lock().unwrap().push((g.into(), d));
        Ok(42)
    }
}

#[tokio::test]
async fn create_generates_uuid_and_saves_timestamped_log() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageAuditLogsService::new(r.clone());
    let cmd = CreateAuditLogCommand {
        guild_id: "g".into(),
        event_type: "test_event".into(),
        actor_id: Some("a".into()),
        actor_name: Some("Actor".into()),
        target_id: Some("t".into()),
        target_name: Some("Target".into()),
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({"key":"val"}),
    };
    let log = svc.create(cmd).await.unwrap();
    assert_eq!(log.event_type, "test_event");
    assert_eq!(log.actor_id.as_deref(), Some("a"));
    assert!(!log.id.is_nil());
    assert_eq!(r.saved.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn list_forwards_filters() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageAuditLogsService::new(r.clone());
    let f = AuditLogFilters {
        event_type: Some("x".into()),
        limit: 50,
        offset: 10,
        ..Default::default()
    };
    svc.list(Some("guild-1"), f).await.unwrap();
    let calls = r.find_calls.lock().unwrap();
    assert_eq!(calls[0].0.as_deref(), Some("guild-1"));
    assert_eq!(calls[0].1, 50);
    assert_eq!(calls[0].2, 10);
}

#[tokio::test]
async fn delete_older_than_days_returns_count() {
    let r = Arc::new(MockRepo::default());
    let svc = ManageAuditLogsService::new(r.clone());
    let n = svc.delete_older_than_days("g", 30).await.unwrap();
    assert_eq!(n, 42);
    assert_eq!(r.deletions.lock().unwrap()[0], ("g".into(), 30));
}
