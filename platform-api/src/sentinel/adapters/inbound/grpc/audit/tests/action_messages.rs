use super::*;
use async_trait::async_trait;
use chrono::TimeZone;
use platform_core::sentinel::domain::errors::DomainError;
use std::sync::Mutex;

#[derive(Default)]
struct MockUc {
    registered: Mutex<Vec<NewDiscordActionMessage>>,
    list: Mutex<Vec<DiscordActionMessage>>,
    fail: bool,
}

#[async_trait]
impl ManageDiscordActionMessagesUseCase for MockUc {
    async fn register(&self, msg: NewDiscordActionMessage) -> Result<(), DomainError> {
        if self.fail {
            return Err(DomainError::Internal("pg down".into()));
        }
        self.registered.lock().unwrap().push(msg);
        Ok(())
    }
    async fn list_for_action(
        &self,
        _action_id: uuid::Uuid,
    ) -> Result<Vec<DiscordActionMessage>, DomainError> {
        Ok(self.list.lock().unwrap().clone())
    }
}

fn grpc(uc: Arc<MockUc>) -> DiscordActionMessagesGrpc {
    DiscordActionMessagesGrpc { uc }
}

#[tokio::test]
async fn register_forwards_fields() {
    let uc = Arc::new(MockUc::default());
    let g = grpc(uc.clone());
    g.register(Request::new(proto::RegisterRequest {
        action_id: uuid::Uuid::nil().to_string(),
        kind: "automod_review".into(),
        guild_id: "g1".into(),
        channel_id: "c1".into(),
        message_id: "m1".into(),
    }))
    .await
    .unwrap();
    let saved = uc.registered.lock().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].kind, "automod_review");
    assert_eq!(saved[0].guild_id.as_str(), "g1");
    assert_eq!(saved[0].message_id.as_str(), "m1");
}

#[tokio::test]
async fn register_rejects_bad_uuid() {
    let g = grpc(Arc::new(MockUc::default()));
    let err = g
        .register(Request::new(proto::RegisterRequest {
            action_id: "not-a-uuid".into(),
            kind: "ticket".into(),
            guild_id: "g".into(),
            channel_id: "c".into(),
            message_id: "m".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn register_maps_domain_error() {
    let uc = Arc::new(MockUc {
        fail: true,
        ..Default::default()
    });
    let err = grpc(uc)
        .register(Request::new(proto::RegisterRequest {
            action_id: uuid::Uuid::nil().to_string(),
            kind: "ticket".into(),
            guild_id: "g".into(),
            channel_id: "c".into(),
            message_id: "m".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::Internal);
}

#[tokio::test]
async fn list_for_action_maps_entries() {
    let uc = Arc::new(MockUc::default());
    uc.list.lock().unwrap().push(DiscordActionMessage {
        action_id: uuid::Uuid::nil(),
        kind: "voice_panel".into(),
        guild_id: "g1".into(),
        channel_id: "c1".into(),
        message_id: "m1".into(),
        posted_at: chrono::Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
        last_edited_at: None,
    });
    let list = grpc(uc)
        .list_for_action(Request::new(proto::ListForActionRequest {
            action_id: uuid::Uuid::nil().to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(list.messages.len(), 1);
    assert_eq!(list.messages[0].kind, "voice_panel");
    assert_eq!(list.messages[0].channel_id, "c1");
    assert_eq!(list.messages[0].posted_at, "2026-01-02T03:04:05+00:00");
    assert!(list.messages[0].last_edited_at.is_none());
}

#[tokio::test]
async fn list_for_action_rejects_bad_uuid() {
    let err = grpc(Arc::new(MockUc::default()))
        .list_for_action(Request::new(proto::ListForActionRequest {
            action_id: "bad".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}
