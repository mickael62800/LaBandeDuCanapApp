use super::*;

use chrono::TimeZone;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_ticket() -> Ticket {
    Ticket {
        id: Uuid::nil(),
        title: "Bug critical".into(),
        status: "open".into(),
        priority: "high".into(),
        author_id: "u1".into(),
        author_name: "Joe".into(),
        assigned_to: Some("mod1".into()),
        server: "main".into(),
        guild_id: Some("123456789012345678".into()),
        category: "bug".into(),
        ticket_type: "support".into(),
        channel_id: Some("c1".into()),
        voice_channel_id: None,
        invited_user_id: None,
        created_at: ts(),
        updated_at: ts(),
        messages_count: 5,
    }
}

#[test]
fn ticket_to_proto_full_mapping() {
    let p = ticket_to_proto(sample_ticket());
    assert_eq!(p.title, "Bug critical");
    assert_eq!(p.status, "open");
    assert_eq!(p.priority, "high");
    assert_eq!(p.assigned_to.as_deref(), Some("mod1"));
    assert_eq!(p.channel_id.as_deref(), Some("c1"));
    assert!(p.voice_channel_id.is_none());
    assert_eq!(p.messages_count, 5);
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn ticket_to_proto_unassigned() {
    let mut t = sample_ticket();
    t.assigned_to = None;
    t.invited_user_id = None;
    let p = ticket_to_proto(t);
    assert!(p.assigned_to.is_none());
    assert!(p.invited_user_id.is_none());
}

#[test]
fn ticket_message_to_proto_mapping() {
    let m = TicketMessage {
        id: Uuid::nil(),
        ticket_id: Uuid::nil(),
        author_name: "Joe".into(),
        author_role: "user".into(),
        content: "Help!".into(),
        created_at: ts(),
    };
    let p = ticket_message_to_proto(m);
    assert_eq!(p.author_name, "Joe");
    assert_eq!(p.author_role, "user");
    assert_eq!(p.content, "Help!");
    assert_eq!(p.created_at, ts().to_rfc3339());
}

#[test]
fn ticket_detail_to_proto_includes_ticket_and_messages() {
    let detail = TicketDetail {
        ticket: sample_ticket(),
        messages: vec![
            TicketMessage {
                id: Uuid::nil(),
                ticket_id: Uuid::nil(),
                author_name: "Joe".into(),
                author_role: "user".into(),
                content: "msg1".into(),
                created_at: ts(),
            },
            TicketMessage {
                id: Uuid::nil(),
                ticket_id: Uuid::nil(),
                author_name: "Mod".into(),
                author_role: "moderator".into(),
                content: "msg2".into(),
                created_at: ts(),
            },
        ],
    };
    let p = ticket_detail_to_proto(detail);
    assert!(p.ticket.is_some());
    assert_eq!(p.messages.len(), 2);
    assert_eq!(p.messages[1].author_role, "moderator");
}

// ── RPC tests avec mock ──

use async_trait::async_trait;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::system::manage_tickets::AssignTicketCommand;
use sentinel_core::ports::inbound::system::manage_tickets::CreateTicketCommand;
use sentinel_core::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use sentinel_core::ports::inbound::system::manage_tickets::ReplyTicketCommand;
use sentinel_core::ports::inbound::system::manage_tickets::UpdateTicketChannelCommand;
use std::sync::Arc;
use std::sync::Mutex;
#[derive(Default)]
struct MockTicketsUc {
    list_tickets: Mutex<Vec<Ticket>>,
    list_calls: Mutex<
        Vec<(
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
            i64,
        )>,
    >,
    detail: Mutex<Option<TicketDetail>>,
    create_calls: Mutex<Vec<CreateTicketCommand>>,
    reply_calls: Mutex<Vec<ReplyTicketCommand>>,
    close_calls: Mutex<Vec<String>>,
    assign_calls: Mutex<Vec<AssignTicketCommand>>,
    update_status_calls: Mutex<Vec<(String, String)>>,
    update_chan_calls: Mutex<Vec<UpdateTicketChannelCommand>>,
    update_prio_calls: Mutex<Vec<(Uuid, String)>>,
    update_sla_calls: Mutex<Vec<(Uuid, Option<String>, Option<String>, Option<i32>)>>,
}

#[async_trait]
impl ManageTicketsUseCase for MockTicketsUc {
    async fn list_tickets(
        &self,
        s: Option<String>,
        p: Option<String>,
        sch: Option<String>,
        a: Option<String>,
        l: i64,
        o: i64,
    ) -> Result<Vec<Ticket>, DomainError> {
        self.list_calls.lock().unwrap().push((s, p, sch, a, l, o));
        Ok(self.list_tickets.lock().unwrap().clone())
    }
    async fn get_ticket_detail(&self, _: &str) -> Result<TicketDetail, DomainError> {
        self.detail
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| DomainError::NotFound("ticket".into()))
    }
    async fn create_ticket(&self, cmd: CreateTicketCommand) -> Result<Ticket, DomainError> {
        let t = sample_ticket();
        self.create_calls.lock().unwrap().push(cmd);
        Ok(t)
    }
    async fn reply_ticket(&self, cmd: ReplyTicketCommand) -> Result<(), DomainError> {
        self.reply_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn close_ticket(&self, id: &str) -> Result<bool, DomainError> {
        self.close_calls.lock().unwrap().push(id.into());
        Ok(true)
    }
    async fn assign_ticket(&self, cmd: AssignTicketCommand) -> Result<(), DomainError> {
        self.assign_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn update_status(&self, id: &str, s: &str) -> Result<(), DomainError> {
        self.update_status_calls
            .lock()
            .unwrap()
            .push((id.into(), s.into()));
        Ok(())
    }
    async fn update_ticket_channel(
        &self,
        cmd: UpdateTicketChannelCommand,
    ) -> Result<(), DomainError> {
        self.update_chan_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn update_priority(&self, id: Uuid, p: &str) -> Result<(), DomainError> {
        self.update_prio_calls.lock().unwrap().push((id, p.into()));
        Ok(())
    }
    async fn update_sla(
        &self,
        id: Uuid,
        f: Option<&str>,
        r: Option<&str>,
        s: Option<i32>,
    ) -> Result<(), DomainError> {
        self.update_sla_calls.lock().unwrap().push((
            id,
            f.map(String::from),
            r.map(String::from),
            s,
        ));
        Ok(())
    }
    async fn bulk_delete_tickets(
        &self,
        _author_id: Option<&str>,
        _from: Option<chrono::DateTime<chrono::Utc>>,
        _to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
}

fn grpc(uc: Arc<MockTicketsUc>) -> TicketsGrpc {
    TicketsGrpc { tickets_uc: uc }
}

#[tokio::test]
async fn list_tickets_default_limit_when_zero() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .list_tickets(Request::new(proto::ListTicketsRequest {
            status: None,
            priority: None,
            search: None,
            author_id: None,
            limit: 0,
            offset: -5,
        }))
        .await
        .unwrap();
    let calls = uc.list_calls.lock().unwrap();
    assert_eq!(calls[0].4, 50); // default 50
    assert_eq!(calls[0].5, 0); // offset floored to 0
}

#[tokio::test]
async fn list_tickets_caps_limit_at_200() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .list_tickets(Request::new(proto::ListTicketsRequest {
            status: Some("open".into()),
            priority: None,
            search: None,
            author_id: None,
            limit: 5000,
            offset: 10,
        }))
        .await
        .unwrap();
    let calls = uc.list_calls.lock().unwrap();
    assert_eq!(calls[0].4, 200); // capped
    assert_eq!(calls[0].0.as_deref(), Some("open"));
}

#[tokio::test]
async fn create_ticket_delegates_command() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .create_ticket(Request::new(proto::CreateTicketRequest {
            title: "Bug".into(),
            priority: "high".into(),
            author_id: "a".into(),
            author_name: "Alice".into(),
            server: "main".into(),
            category: "bug".into(),
            ticket_type: "support".into(),
            channel_id: Some("c".into()),
            guild_id: Some("123456789012345678".into()),
        }))
        .await
        .unwrap();
    let calls = uc.create_calls.lock().unwrap();
    assert_eq!(calls[0].title, "Bug");
    assert_eq!(calls[0].author_name, "Alice");
}

#[tokio::test]
async fn reply_ticket_delegates_command() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .reply_ticket(Request::new(proto::ReplyTicketRequest {
            ticket_id: "t1".into(),
            content: "msg".into(),
            author_name: "Joe".into(),
            author_role: "user".into(),
        }))
        .await
        .unwrap();
    assert_eq!(uc.reply_calls.lock().unwrap()[0].content, "msg");
}

#[tokio::test]
async fn close_ticket_delegates_id() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .close_ticket(Request::new(proto::CloseTicketRequest { id: "t1".into() }))
        .await
        .unwrap();
    assert_eq!(uc.close_calls.lock().unwrap()[0], "t1");
}

#[tokio::test]
async fn update_status_delegates() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .update_status(Request::new(proto::UpdateStatusRequest {
            id: "t1".into(),
            status: "resolved".into(),
        }))
        .await
        .unwrap();
    let calls = uc.update_status_calls.lock().unwrap();
    assert_eq!(calls[0], ("t1".into(), "resolved".into()));
}

#[tokio::test]
async fn assign_ticket_delegates() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .assign_ticket(Request::new(proto::AssignTicketRequest {
            ticket_id: "t1".into(),
            assignee: "mod1".into(),
        }))
        .await
        .unwrap();
    let calls = uc.assign_calls.lock().unwrap();
    assert_eq!(calls[0].assignee, "mod1");
}

#[tokio::test]
async fn update_priority_rejects_invalid_uuid() {
    let g = grpc(Arc::new(MockTicketsUc::default()));
    let err = g
        .update_priority(Request::new(proto::UpdatePriorityRequest {
            id: "not-a-uuid".into(),
            priority: "high".into(),
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn update_priority_valid_uuid_delegates() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let id = Uuid::new_v4();
    let _ = g
        .update_priority(Request::new(proto::UpdatePriorityRequest {
            id: id.to_string(),
            priority: "high".into(),
        }))
        .await
        .unwrap();
    let calls = uc.update_prio_calls.lock().unwrap();
    assert_eq!(calls[0].0, id);
}

#[tokio::test]
async fn update_sla_invalid_uuid_rejected() {
    let g = grpc(Arc::new(MockTicketsUc::default()));
    let err = g
        .update_sla(Request::new(proto::UpdateSlaRequest {
            id: "bad".into(),
            first_response_at: None,
            resolved_at: None,
            satisfaction_rating: None,
        }))
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn update_sla_all_fields_delegate() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let id = Uuid::new_v4();
    let _ = g
        .update_sla(Request::new(proto::UpdateSlaRequest {
            id: id.to_string(),
            first_response_at: Some("2026-01-01T00:00:00Z".into()),
            resolved_at: Some("2026-01-02T00:00:00Z".into()),
            satisfaction_rating: Some(5),
        }))
        .await
        .unwrap();
    let calls = uc.update_sla_calls.lock().unwrap();
    assert_eq!(calls[0].0, id);
    assert_eq!(calls[0].3, Some(5));
}

#[tokio::test]
async fn update_ticket_channel_delegates_optionals() {
    let uc = Arc::new(MockTicketsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .update_ticket_channel(Request::new(proto::UpdateTicketChannelRequest {
            ticket_id: "t".into(),
            voice_channel_id: Some("vc".into()),
            invited_user_id: None,
        }))
        .await
        .unwrap();
    let calls = uc.update_chan_calls.lock().unwrap();
    assert_eq!(calls[0].voice_channel_id.as_deref(), Some("vc"));
    assert!(calls[0].invited_user_id.is_none());
}
