use super::*;
use async_trait::async_trait;

struct StubCounter {
    counts: Vec<(String, u64)>,
    expected_days: u32,
}

#[async_trait]
impl AuditEventCounter for StubCounter {
    async fn count_by_event_type(
        &self,
        _guild_id: &str,
        days: u32,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        assert_eq!(days, self.expected_days);
        Ok(self.counts.clone())
    }
}

struct FailingCounter;

#[async_trait]
impl AuditEventCounter for FailingCounter {
    async fn count_by_event_type(
        &self,
        _guild_id: &str,
        _days: u32,
    ) -> Result<Vec<(String, u64)>, DomainError> {
        Err(DomainError::Internal("boom".into()))
    }
}

#[tokio::test]
async fn aggregates_over_seven_days() {
    let counter = Arc::new(StubCounter {
        counts: vec![
            ("member_join".into(), 4),
            ("member_ban".into(), 1),
            ("voice_join".into(), 10),
        ],
        expected_days: 7,
    });
    let service = GetWeeklyReportService::new(counter);

    let report = service.get("123").await.unwrap();

    assert_eq!(report.member_joins, 4);
    assert_eq!(report.bans, 1);
    assert_eq!(report.voice_events, 10);
    assert_eq!(report.messages_deleted, 0);
}

#[tokio::test]
async fn propagates_counter_error() {
    let service = GetWeeklyReportService::new(Arc::new(FailingCounter));
    let err = service.get("123").await.unwrap_err();
    assert!(matches!(err, DomainError::Internal(_)));
}
