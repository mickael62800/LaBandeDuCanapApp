use super::*;

use chrono::TimeZone;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

#[test]
fn security_event_to_proto_full_mapping() {
    let e = SecurityEvent {
        id: Uuid::nil(),
        guild_id: "g".into(),
        event_type: "raid".into(),
        severity: "critical".into(),
        description: "Mass join detected".into(),
        user_ids: vec!["u1".into(), "u2".into(), "u3".into()],
        created_at: ts(),
    };
    let p = security_event_to_proto(e);
    assert_eq!(p.id, Uuid::nil().to_string());
    assert_eq!(p.guild_id, "g");
    assert_eq!(p.event_type, "raid");
    assert_eq!(p.severity, "critical");
    assert_eq!(p.description, "Mass join detected");
    assert_eq!(p.user_ids.len(), 3);
    assert_eq!(p.user_ids[1], "u2");
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn security_event_to_proto_no_users() {
    let e = SecurityEvent {
        id: Uuid::nil(),
        guild_id: "g".into(),
        event_type: "scan".into(),
        severity: "info".into(),
        description: String::new(),
        user_ids: vec![],
        created_at: ts(),
    };
    let p = security_event_to_proto(e);
    assert!(p.user_ids.is_empty());
    assert_eq!(p.severity, "info");
}

// ── RPC handlers avec mock ManageSecurityUseCase ──

use async_trait::async_trait;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::audit::manage_security::AnalyzeNewMemberCommand;
use platform_core::sentinel::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_security::ReportSecurityEventCommand;
use platform_core::sentinel::ports::inbound::audit::manage_security::SecurityDecision;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
struct MockSecurityUc {
    report_calls: Mutex<Vec<ReportSecurityEventCommand>>,
    list_returns: Mutex<Vec<SecurityEvent>>,
    list_calls: Mutex<Vec<Option<String>>>,
    analyze_returns: Mutex<SecurityDecision>,
}

#[async_trait]
impl ManageSecurityUseCase for MockSecurityUc {
    async fn report_event(
        &self,
        cmd: ReportSecurityEventCommand,
    ) -> Result<SecurityEvent, DomainError> {
        let event = SecurityEvent {
            id: Uuid::new_v4(),
            guild_id: cmd.guild_id.clone(),
            event_type: cmd.event_type.clone(),
            severity: cmd.severity.clone(),
            description: cmd.description.clone(),
            user_ids: cmd.user_ids.clone(),
            created_at: chrono::Utc::now(),
        };
        self.report_calls.lock().unwrap().push(cmd);
        Ok(event)
    }
    async fn purge_events(&self, _: &str) -> Result<(u64, u64), DomainError> {
        Ok((0, 0))
    }
    async fn list_events(&self, guild_id: Option<&str>) -> Result<Vec<SecurityEvent>, DomainError> {
        self.list_calls
            .lock()
            .unwrap()
            .push(guild_id.map(String::from));
        Ok(self.list_returns.lock().unwrap().clone())
    }
    async fn analyze_new_member(
        &self,
        _: AnalyzeNewMemberCommand,
    ) -> Result<SecurityDecision, DomainError> {
        Ok(self.analyze_returns.lock().unwrap().clone())
    }
}

fn grpc(uc: Arc<MockSecurityUc>) -> SecurityGrpc {
    SecurityGrpc { uc }
}

#[tokio::test]
async fn report_event_returns_persisted_event() {
    let uc = Arc::new(MockSecurityUc::default());
    let g = grpc(uc.clone());
    let resp = g
        .report_event(Request::new(proto::ReportEventRequest {
            guild_id: "g1".into(),
            event_type: "raid".into(),
            severity: "critical".into(),
            description: "mass join".into(),
            user_ids: vec!["u1".into(), "u2".into()],
        }))
        .await
        .unwrap();
    let event = resp.into_inner();
    assert_eq!(event.guild_id, "g1");
    assert_eq!(event.event_type, "raid");
    assert_eq!(event.user_ids.len(), 2);
    assert_eq!(uc.report_calls.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn list_events_with_guild_id_filter() {
    let uc = Arc::new(MockSecurityUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .list_events(Request::new(proto::ListEventsRequest {
            guild_id: Some("g1".into()),
        }))
        .await
        .unwrap();
    let calls = uc.list_calls.lock().unwrap();
    assert_eq!(calls[0].as_deref(), Some("g1"));
}

#[tokio::test]
async fn list_events_none_filter_returns_all_guilds() {
    let uc = Arc::new(MockSecurityUc::default());
    uc.list_returns.lock().unwrap().push(SecurityEvent {
        id: Uuid::new_v4(),
        guild_id: "x".into(),
        event_type: "scan".into(),
        severity: "info".into(),
        description: String::new(),
        user_ids: vec![],
        created_at: chrono::Utc::now(),
    });
    let g = grpc(uc.clone());
    let resp = g
        .list_events(Request::new(proto::ListEventsRequest { guild_id: None }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().events.len(), 1);
    assert!(uc.list_calls.lock().unwrap()[0].is_none());
}

#[tokio::test]
async fn analyze_new_member_delegates_and_maps_decision() {
    let uc = Arc::new(MockSecurityUc::default());
    *uc.analyze_returns.lock().unwrap() = SecurityDecision {
        is_raid: true,
        raid_score: 85,
        is_suspicious_account: false,
        is_alt_account: false,
        alt_similar_to: String::new(),
        quarantine: true,
        send_captcha: true,
        activate_lockdown: true,
        slowmode_secs: 30,
        suggest_only: false,
        event_type: "raid_detected".into(),
        event_description: "Raid pattern".into(),
    };
    let g = grpc(uc);
    let resp = g
        .analyze_new_member(Request::new(proto::AnalyzeNewMemberRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "newbie".into(),
            has_avatar: false,
            account_created_timestamp: 0,
            is_bot: false,
            is_velocity_raid: false,
            recent_joins: vec![],
        }))
        .await
        .unwrap();
    let d = resp.into_inner();
    assert!(d.is_raid);
    assert_eq!(d.raid_score, 85);
    assert!(d.quarantine);
    assert_eq!(d.event_type, "raid_detected");
}

#[tokio::test]
async fn analyze_new_member_maps_recent_joins_to_domain() {
    let uc = Arc::new(MockSecurityUc::default());
    let g = grpc(uc);
    // Ne doit pas panic avec recent_joins populé.
    let _ = g
        .analyze_new_member(Request::new(proto::AnalyzeNewMemberRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "x".into(),
            has_avatar: true,
            account_created_timestamp: 1700000000,
            is_bot: false,
            is_velocity_raid: false,
            recent_joins: vec![
                proto::RecentJoinEntry {
                    username: "bot1".into(),
                    has_avatar: false,
                    account_created_timestamp: 1700000001,
                },
                proto::RecentJoinEntry {
                    username: "bot2".into(),
                    has_avatar: false,
                    account_created_timestamp: 1700000002,
                },
            ],
        }))
        .await
        .unwrap();
}
