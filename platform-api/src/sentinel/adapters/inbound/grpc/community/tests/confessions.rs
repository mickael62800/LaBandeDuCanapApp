use super::*;
use async_trait::async_trait;
use chrono::TimeZone;
use platform_core::sentinel::domain::entities::community::confession::ConfessionReport;
use platform_core::sentinel::domain::entities::community::confession::ReportStatus;
use platform_core::sentinel::domain::errors::DomainError;
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Default)]
struct MockUc {
    created: Mutex<Vec<CreateConfessionCommand>>,
    refs: Mutex<Vec<(Uuid, String, String, Option<String>)>>,
    replies: Mutex<Vec<CreateReplyCommand>>,
    reports: Mutex<Vec<CreateReportCommand>>,
    deleted: Mutex<Vec<(Uuid, String, Option<String>)>>,
}

fn sample_confession() -> Confession {
    Confession {
        id: Uuid::nil(),
        guild_id: "g1".into(),
        public_number: 42,
        author_user_id: "u1".into(),
        content: "coucou".into(),
        message_id: Some("m1".into()),
        channel_id: Some("c1".into()),
        thread_id: Some("t1".into()),
        deleted_at: None,
        deleted_by: None,
        deleted_reason: None,
        edited_at: None,
        created_at: chrono::Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
    }
}

#[async_trait]
impl ManageConfessionsUseCase for MockUc {
    async fn create(&self, cmd: CreateConfessionCommand) -> Result<Confession, DomainError> {
        self.created.lock().unwrap().push(cmd);
        Ok(sample_confession())
    }
    async fn update_message_refs(
        &self,
        id: Uuid,
        message_id: String,
        channel_id: String,
        thread_id: Option<String>,
    ) -> Result<(), DomainError> {
        self.refs
            .lock()
            .unwrap()
            .push((id, message_id, channel_id, thread_id));
        Ok(())
    }
    async fn edit_content(&self, _: Uuid, _: &str, _: String) -> Result<Confession, DomainError> {
        unimplemented!()
    }
    async fn delete(
        &self,
        id: Uuid,
        deleted_by: String,
        reason: Option<String>,
    ) -> Result<Confession, DomainError> {
        self.deleted.lock().unwrap().push((id, deleted_by, reason));
        Ok(sample_confession())
    }
    async fn get(&self, _: Uuid) -> Result<Confession, DomainError> {
        Ok(sample_confession())
    }
    async fn get_by_message_id(&self, _: &str) -> Result<Option<Confession>, DomainError> {
        unimplemented!()
    }
    async fn get_by_public_number(&self, _: &str, _: i32) -> Result<Confession, DomainError> {
        unimplemented!()
    }
    async fn list(&self, _: &str, _: i64, _: bool) -> Result<Vec<Confession>, DomainError> {
        Ok(vec![sample_confession()])
    }
    async fn create_reply(&self, cmd: CreateReplyCommand) -> Result<ConfessionReply, DomainError> {
        self.replies.lock().unwrap().push(cmd);
        Ok(ConfessionReply {
            id: Uuid::nil(),
            confession_id: Uuid::nil(),
            public_number: 7,
            author_user_id: "u2".into(),
            content: "re".into(),
            is_anonymous: true,
            message_id: None,
            deleted_at: None,
            deleted_by: None,
            edited_at: None,
            created_at: chrono::Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
        })
    }
    async fn update_reply_message_id(&self, _: Uuid, _: String) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_reply(&self, _: Uuid, _: String) -> Result<ConfessionReply, DomainError> {
        unimplemented!()
    }
    async fn list_replies(&self, _: Uuid) -> Result<Vec<ConfessionReply>, DomainError> {
        unimplemented!()
    }
    async fn get_reply_parent_guild(&self, _: Uuid) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn create_report(
        &self,
        cmd: CreateReportCommand,
    ) -> Result<ConfessionReport, DomainError> {
        self.reports.lock().unwrap().push(cmd);
        Ok(ConfessionReport {
            id: Uuid::nil(),
            guild_id: "g1".into(),
            confession_id: Some(Uuid::nil()),
            reply_id: None,
            reporter_user_id: "u3".into(),
            reason: "spam".into(),
            status: ReportStatus::Pending,
            resolved_by: None,
            resolved_at: None,
            created_at: chrono::Utc::now(),
        })
    }
    async fn list_reports(
        &self,
        _: &str,
        _: Option<ReportStatus>,
        _: i64,
    ) -> Result<Vec<ConfessionReport>, DomainError> {
        unimplemented!()
    }
    async fn get_report_guild(&self, _: Uuid) -> Result<String, DomainError> {
        unimplemented!()
    }
    async fn resolve_report(&self, _: Uuid, _: ReportStatus, _: String) -> Result<(), DomainError> {
        unimplemented!()
    }
    async fn get_config(&self, guild_id: &str) -> Result<ConfessionConfig, DomainError> {
        Ok(ConfessionConfig::defaults(guild_id.to_string()))
    }
    async fn save_config(&self, _: ConfessionConfig) -> Result<ConfessionConfig, DomainError> {
        unimplemented!()
    }
}

fn grpc(uc: Arc<MockUc>) -> ConfessionsGrpc {
    ConfessionsGrpc { uc }
}

#[tokio::test]
async fn create_forwards_and_maps() {
    let uc = Arc::new(MockUc::default());
    let resp = grpc(uc.clone())
        .create_confession(Request::new(proto::CreateConfessionRequest {
            guild_id: "g1".into(),
            author_user_id: "u1".into(),
            content: "hello".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(uc.created.lock().unwrap()[0].content, "hello");
    assert_eq!(resp.public_number, 42);
    assert_eq!(resp.thread_id.as_deref(), Some("t1"));
    assert_eq!(resp.created_at, "2026-01-02T03:04:05+00:00");
}

#[tokio::test]
async fn update_message_refs_forwards() {
    let uc = Arc::new(MockUc::default());
    grpc(uc.clone())
        .update_message_refs(Request::new(proto::UpdateMessageRefsRequest {
            id: Uuid::nil().to_string(),
            message_id: "m9".into(),
            channel_id: "c9".into(),
            thread_id: Some("t9".into()),
        }))
        .await
        .unwrap();
    let refs = uc.refs.lock().unwrap();
    assert_eq!(refs[0].1, "m9");
    assert_eq!(refs[0].3.as_deref(), Some("t9"));
}

#[tokio::test]
async fn create_reply_maps_public_number() {
    let uc = Arc::new(MockUc::default());
    let resp = grpc(uc.clone())
        .create_reply(Request::new(proto::CreateReplyRequest {
            confession_id: Uuid::nil().to_string(),
            author_user_id: "u2".into(),
            content: "re".into(),
            is_anonymous: true,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.public_number, 7);
    assert!(resp.is_anonymous);
    assert!(uc.replies.lock().unwrap()[0].is_anonymous);
}

#[tokio::test]
async fn create_report_parses_optional_ids() {
    let uc = Arc::new(MockUc::default());
    grpc(uc.clone())
        .create_report(Request::new(proto::CreateReportRequest {
            guild_id: "g1".into(),
            confession_id: Some(Uuid::nil().to_string()),
            reply_id: None,
            reporter_user_id: "u3".into(),
            reason: "spam".into(),
        }))
        .await
        .unwrap();
    let reports = uc.reports.lock().unwrap();
    assert!(reports[0].confession_id.is_some());
    assert!(reports[0].reply_id.is_none());
}

#[tokio::test]
async fn create_report_rejects_bad_uuid() {
    let err = grpc(Arc::new(MockUc::default()))
        .create_report(Request::new(proto::CreateReportRequest {
            guild_id: "g1".into(),
            confession_id: Some("not-a-uuid".into()),
            reply_id: None,
            reporter_user_id: "u3".into(),
            reason: "spam".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn delete_forwards_reason() {
    let uc = Arc::new(MockUc::default());
    grpc(uc.clone())
        .delete_confession(Request::new(proto::DeleteConfessionRequest {
            id: Uuid::nil().to_string(),
            deleted_by: "admin".into(),
            reason: Some("abus".into()),
        }))
        .await
        .unwrap();
    let del = uc.deleted.lock().unwrap();
    assert_eq!(del[0].1, "admin");
    assert_eq!(del[0].2.as_deref(), Some("abus"));
}

#[tokio::test]
async fn get_config_maps_defaults() {
    let resp = grpc(Arc::new(MockUc::default()))
        .get_config(Request::new(proto::GetConfigRequest {
            guild_id: "g1".into(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(resp.guild_id, "g1");
    assert!(resp.enabled);
    assert_eq!(resp.max_chars, 2000);
}

#[tokio::test]
async fn list_maps_entries() {
    let list = grpc(Arc::new(MockUc::default()))
        .list_confessions(Request::new(proto::ListConfessionsRequest {
            guild_id: "g1".into(),
            limit: 500,
            include_deleted: false,
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.confessions.len(), 1);
    assert_eq!(list.confessions[0].public_number, 42);
}
