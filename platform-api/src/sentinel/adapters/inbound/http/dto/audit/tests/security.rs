use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::audit::security_event::SecurityEvent;
use uuid::Uuid;

#[test]
fn report_event_default_user_ids_empty() {
    let dto: ReportEventDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g",
        "event_type": "raid",
        "severity": "high",
        "description": "desc"
    }))
    .unwrap();
    assert!(dto.user_ids.is_empty());
}

#[test]
fn report_event_to_command() {
    let dto = ReportEventDto {
        guild_id: "g".into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "d".into(),
        user_ids: vec!["u1".into(), "u2".into()],
    };
    let cmd: ReportSecurityEventCommand = dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.event_type, "raid");
    assert_eq!(cmd.severity, "high");
    assert_eq!(cmd.description, "d");
    assert_eq!(cmd.user_ids.len(), 2);
}

#[test]
fn from_security_event_maps_all_fields() {
    let ev = SecurityEvent {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "raid".into(),
        severity: "high".into(),
        description: "d".into(),
        user_ids: vec!["u".into()],
        created_at: Utc::now(),
    };
    let id = ev.id.to_string();
    let dto = SecurityEventResponseDto::from(ev);
    assert_eq!(dto.id, id);
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.user_ids, vec!["u".to_string()]);
    assert!(dto.created_at.contains('T'));
}

#[test]
fn query_params_optional() {
    let p: SecurityQueryParams = serde_json::from_str("{}").unwrap();
    assert!(p.guild_id.is_none());
    let p: SecurityQueryParams =
        serde_json::from_value(serde_json::json!({"guild_id": "g"})).unwrap();
    assert_eq!(p.guild_id.as_deref(), Some("g"));
}
