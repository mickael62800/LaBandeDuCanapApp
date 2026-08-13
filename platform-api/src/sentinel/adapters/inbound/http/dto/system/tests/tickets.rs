use super::*;

use chrono::Utc;
use uuid::Uuid;

fn make_ticket() -> Ticket {
    Ticket {
        id: Uuid::new_v4(),
        title: "Bug report".into(),
        status: "open".into(),
        priority: "high".into(),
        author_id: "user1".into(),
        author_name: "Alice".into(),
        assigned_to: Some("mod1".into()),
        server: "TestServer".into(),
        guild_id: Some("123456789012345678".into()),
        category: "bug".into(),
        ticket_type: "probleme_serveur".into(),
        channel_id: Some("chan1".into()),
        voice_channel_id: None,
        invited_user_id: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        messages_count: 3,
    }
}

fn make_message(ticket_id: Uuid) -> TicketMessage {
    TicketMessage {
        id: Uuid::new_v4(),
        ticket_id,
        author_name: "Mod".into(),
        author_role: "moderator".into(),
        content: "Looking into it".into(),
        created_at: Utc::now(),
    }
}

// ── Defaults ──

#[test]
fn default_priority_is_medium() {
    assert_eq!(default_priority(), "medium");
}

#[test]
fn default_ticket_type_is_autre() {
    assert_eq!(default_ticket_type(), "autre");
}

#[test]
fn default_author_name_is_staff() {
    assert_eq!(default_author_name(), "staff");
}

#[test]
fn default_author_role_is_moderator() {
    assert_eq!(default_author_role(), "moderator");
}

// ── Ticket → TicketResponseDto ──

#[test]
fn ticket_to_dto_preserves_fields() {
    let t = make_ticket();
    let id = t.id;
    let dto = TicketResponseDto::from(t);
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.title, "Bug report");
    assert_eq!(dto.status, "open");
    assert_eq!(dto.priority, "high");
    assert_eq!(dto.assigned_to, Some("mod1".into()));
    assert_eq!(dto.messages_count, 3);
}

#[test]
fn ticket_to_dto_formats_dates_rfc3339() {
    let dto = TicketResponseDto::from(make_ticket());
    assert!(dto.created_at.contains("T"));
    assert!(dto.updated_at.contains("T"));
}

#[test]
fn ticket_to_dto_none_optionals() {
    let mut t = make_ticket();
    t.assigned_to = None;
    t.channel_id = None;
    t.voice_channel_id = None;
    t.invited_user_id = None;
    let dto = TicketResponseDto::from(t);
    assert!(dto.assigned_to.is_none());
    assert!(dto.channel_id.is_none());
    assert!(dto.voice_channel_id.is_none());
    assert!(dto.invited_user_id.is_none());
}

// ── TicketMessage → TicketMessageDto ──

#[test]
fn message_to_dto_preserves_fields() {
    let tid = Uuid::new_v4();
    let m = make_message(tid);
    let mid = m.id;
    let dto = TicketMessageDto::from(m);
    assert_eq!(dto.id, mid.to_string());
    assert_eq!(dto.ticket_id, tid.to_string());
    assert_eq!(dto.content, "Looking into it");
    assert_eq!(dto.author_role, "moderator");
}

#[test]
fn message_to_dto_formats_date() {
    let dto = TicketMessageDto::from(make_message(Uuid::new_v4()));
    assert!(dto.created_at.contains("T"));
}

// ── TicketDetail → TicketDetailDto ──

#[test]
fn detail_to_dto_aggregates() {
    let t = make_ticket();
    let tid = t.id;
    let detail = TicketDetail {
        ticket: t,
        messages: vec![make_message(tid), make_message(tid)],
    };
    let dto = TicketDetailDto::from(detail);
    assert_eq!(dto.ticket.title, "Bug report");
    assert_eq!(dto.messages.len(), 2);
}

#[test]
fn detail_to_dto_empty_messages() {
    let detail = TicketDetail {
        ticket: make_ticket(),
        messages: vec![],
    };
    let dto = TicketDetailDto::from(detail);
    assert!(dto.messages.is_empty());
}

// ── CreateTicketDto → CreateTicketCommand ──

#[test]
fn create_dto_to_command_maps_all_fields() {
    let dto = CreateTicketDto {
        title: "Test".into(),
        priority: "urgent".into(),
        author_id: "u1".into(),
        author_name: "User".into(),
        server: "srv".into(),
        guild_id: Some("123456789012345678".into()),
        category: "cat".into(),
        ticket_type: "question".into(),
        channel_id: Some("ch1".into()),
    };
    let cmd: CreateTicketCommand = dto.into();
    assert_eq!(cmd.title, "Test");
    assert_eq!(cmd.priority, "urgent");
    assert_eq!(cmd.ticket_type, "question");
    assert_eq!(cmd.channel_id, Some("ch1".into()));
}
