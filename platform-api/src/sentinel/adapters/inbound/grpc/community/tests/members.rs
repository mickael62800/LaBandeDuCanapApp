use super::*;

use chrono::TimeZone;

fn ts() -> DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_member() -> GuildMember {
    GuildMember {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "alice".into(),
        display_name: Some("Alice".into()),
        avatar: Some("hash123".into()),
        roles: serde_json::json!(["role1", "role2"]),
        joined_at: Some(ts()),
        account_created: Some(ts()),
        is_bot: false,
        last_seen_at: Some(ts()),
        left_at: None,
    }
}

#[test]
fn member_to_proto_full_mapping() {
    let p = member_to_proto(sample_member()).unwrap();
    assert_eq!(p.guild_id, "g1");
    assert_eq!(p.user_id, "u1");
    assert_eq!(p.username, "alice");
    assert_eq!(p.display_name.as_deref(), Some("Alice"));
    assert!(p.roles_json.contains("role1"));
    assert_eq!(p.joined_at, Some(ts().to_rfc3339()));
    assert!(!p.is_bot);
}

#[test]
fn member_to_proto_with_none_dates() {
    let mut m = sample_member();
    m.joined_at = None;
    m.account_created = None;
    m.last_seen_at = None;
    m.display_name = None;
    m.avatar = None;
    let p = member_to_proto(m).unwrap();
    assert!(p.joined_at.is_none());
    assert!(p.account_created.is_none());
    assert!(p.last_seen_at.is_none());
    assert!(p.display_name.is_none());
    assert!(p.avatar.is_none());
}

#[test]
fn member_round_trip_via_proto() {
    let original = sample_member();
    let p = member_to_proto(original.clone()).unwrap();
    let back = proto_to_member(p).unwrap();
    assert_eq!(back.guild_id, original.guild_id);
    assert_eq!(back.user_id, original.user_id);
    assert_eq!(back.username, original.username);
    assert_eq!(back.display_name, original.display_name);
    assert_eq!(back.is_bot, original.is_bot);
    assert_eq!(back.joined_at, original.joined_at);
    assert_eq!(back.roles, original.roles);
}

#[test]
fn proto_to_member_invalid_roles_json_falls_back_to_empty_array() {
    let p = proto::GuildMember {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "x".into(),
        display_name: None,
        avatar: None,
        roles_json: "not a json".into(),
        joined_at: None,
        account_created: None,
        is_bot: false,
        last_seen_at: None,
    };
    let m = proto_to_member(p).unwrap();
    assert_eq!(m.roles, serde_json::Value::Array(vec![]));
}

#[test]
fn parse_rfc3339_none_yields_none() {
    assert_eq!(parse_rfc3339(None).unwrap(), None);
}

#[test]
fn parse_rfc3339_valid_date() {
    let s = ts().to_rfc3339();
    let parsed = parse_rfc3339(Some(s)).unwrap();
    assert_eq!(parsed, Some(ts()));
}

#[test]
fn parse_rfc3339_invalid_returns_invalid_argument() {
    let err = parse_rfc3339(Some("not-a-date".into())).unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("date"));
}

// ── RPC tests avec mock ──

use async_trait::async_trait;
use platform_core::sentinel::domain::entities::community::guild_member::MemberSummary;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_members::ManageMembersUseCase;
use platform_core::sentinel::ports::inbound::community::manage_members::RegisterMemberCommand;
use platform_core::sentinel::ports::inbound::community::manage_members::SyncMembersCommand;
use platform_core::sentinel::ports::inbound::community::manage_members::UpdateMemberCommand;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Default)]
struct MockMembersUc {
    member: Mutex<Option<GuildMember>>,
    sync_calls: Mutex<Vec<SyncMembersCommand>>,
    sync_return: Mutex<u64>,
    register_calls: Mutex<Vec<RegisterMemberCommand>>,
    remove_calls: Mutex<Vec<(String, String)>>,
    update_calls: Mutex<Vec<UpdateMemberCommand>>,
}

#[async_trait]
impl ManageMembersUseCase for MockMembersUc {
    async fn list_members(&self, _: &str) -> Result<Vec<GuildMember>, DomainError> {
        Ok(vec![])
    }
    async fn get_member(&self, _: &str, _: &str) -> Result<GuildMember, DomainError> {
        self.member
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| DomainError::NotFound("member".into()))
    }
    async fn get_member_summary(&self, _: &str, _: &str) -> Result<MemberSummary, DomainError> {
        unimplemented!()
    }
    async fn sync_members(&self, cmd: SyncMembersCommand) -> Result<u64, DomainError> {
        self.sync_calls.lock().unwrap().push(cmd);
        Ok(*self.sync_return.lock().unwrap())
    }
    async fn register_member(&self, cmd: RegisterMemberCommand) -> Result<(), DomainError> {
        self.register_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn remove_member(&self, g: &str, u: &str) -> Result<(), DomainError> {
        self.remove_calls.lock().unwrap().push((g.into(), u.into()));
        Ok(())
    }
    async fn update_member(&self, cmd: UpdateMemberCommand) -> Result<(), DomainError> {
        self.update_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn reset_member(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<(&'static str, u64)>, DomainError> {
        Ok(vec![])
    }
    async fn leave_member(&self, _: &str, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn rejoin_member(&self, _: &str, _: &str) -> Result<u64, DomainError> {
        Ok(0)
    }

    // Lectures servant la page membre : hors du perimetre de ces tests.
    async fn upcoming_anniversaries(
        &self,
        _: &str,
        _: i32,
    ) -> Result<
        Vec<platform_core::sentinel::domain::entities::community::milestone::JoinAnniversary>,
        DomainError,
    > {
        Ok(vec![])
    }

    async fn recent_joins(&self, _: &str, _: i32, _: i64) -> Result<Vec<GuildMember>, DomainError> {
        Ok(vec![])
    }
}

fn grpc(uc: Arc<MockMembersUc>) -> MembersGrpc {
    MembersGrpc { uc }
}

#[tokio::test]
async fn get_member_returns_none_on_not_found() {
    let g = grpc(Arc::new(MockMembersUc::default()));
    let resp = g
        .get_member(Request::new(proto::GetMemberRequest {
            guild_id: "g".into(),
            user_id: "ghost".into(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().member.is_none());
}

#[tokio::test]
async fn get_member_returns_some_when_found() {
    let uc = Arc::new(MockMembersUc::default());
    *uc.member.lock().unwrap() = Some(sample_member());
    let g = grpc(uc);
    let resp = g
        .get_member(Request::new(proto::GetMemberRequest {
            guild_id: "g1".into(),
            user_id: "u1".into(),
        }))
        .await
        .unwrap();
    let m = resp.into_inner().member.unwrap();
    assert_eq!(m.user_id, "u1");
}

#[tokio::test]
async fn sync_members_returns_count() {
    let uc = Arc::new(MockMembersUc::default());
    *uc.sync_return.lock().unwrap() = 42;
    let g = grpc(uc.clone());
    let resp = g
        .sync_members(Request::new(proto::SyncMembersRequest {
            guild_id: "g".into(),
            members: vec![],
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().synced_count, 42);
    assert_eq!(uc.sync_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn register_member_missing_body_returns_invalid_argument() {
    let g = grpc(Arc::new(MockMembersUc::default()));
    let err = g
        .register_member(Request::new(proto::RegisterMemberRequest { member: None }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("member manquant"));
}

#[tokio::test]
async fn register_member_delegates_to_uc() {
    let uc = Arc::new(MockMembersUc::default());
    let g = grpc(uc.clone());
    let proto_m = member_to_proto(sample_member()).unwrap();
    let _ = g
        .register_member(Request::new(proto::RegisterMemberRequest {
            member: Some(proto_m),
        }))
        .await
        .unwrap();
    assert_eq!(uc.register_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn remove_member_delegates() {
    let uc = Arc::new(MockMembersUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .remove_member(Request::new(proto::RemoveMemberRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    assert_eq!(uc.remove_calls.lock().unwrap()[0], ("g".into(), "u".into()));
}

#[tokio::test]
async fn update_member_invalid_roles_json_returns_error() {
    let g = grpc(Arc::new(MockMembersUc::default()));
    let err = g
        .update_member(Request::new(proto::UpdateMemberRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: None,
            display_name: None,
            avatar: None,
            roles_json: Some("not-json".into()),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
    assert!(err.message().contains("roles_json"));
}

#[tokio::test]
async fn update_member_valid_roles_json_delegates() {
    let uc = Arc::new(MockMembersUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .update_member(Request::new(proto::UpdateMemberRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: Some("new".into()),
            display_name: None,
            avatar: None,
            roles_json: Some(r#"["r1","r2"]"#.into()),
        }))
        .await
        .unwrap();
    let calls = uc.update_calls.lock().unwrap();
    assert_eq!(calls[0].username.as_deref(), Some("new"));
    assert!(calls[0].roles.is_some());
}

#[tokio::test]
async fn update_member_no_roles_json_is_ok() {
    let uc = Arc::new(MockMembersUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .update_member(Request::new(proto::UpdateMemberRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: None,
            display_name: None,
            avatar: None,
            roles_json: None,
        }))
        .await
        .unwrap();
    assert!(uc.update_calls.lock().unwrap()[0].roles.is_none());
}
