use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::audit::audit_log::AuditLog;
use uuid::Uuid;

#[test]
fn default_details_is_empty_object_when_missing() {
    let dto: CreateAuditLogDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "event_type": "x"
    }))
    .unwrap();
    assert_eq!(dto.details, serde_json::json!({}));
    assert!(dto.actor_id.is_none());
    assert!(dto.target_id.is_none());
}

#[test]
fn create_dto_to_command_preserves_all_fields() {
    let dto = CreateAuditLogDto {
        guild_id: "g".into(),
        event_type: "role.update".into(),
        actor_id: Some("a".into()),
        actor_name: Some("Admin".into()),
        target_id: Some("t".into()),
        target_name: Some("Target".into()),
        channel_id: Some("c".into()),
        channel_name: Some("general".into()),
        details: serde_json::json!({"from": "x"}),
    };
    let cmd: CreateAuditLogCommand = dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.event_type, "role.update");
    assert_eq!(cmd.actor_name.as_deref(), Some("Admin"));
    assert_eq!(cmd.details["from"], "x");
}

#[test]
fn from_audit_log_maps_all_fields() {
    let log = AuditLog {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "ban".into(),
        actor_id: Some("a".into()),
        actor_name: None,
        target_id: None,
        target_name: Some("bob".into()),
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({}),
        created_at: Utc::now(),
    };
    let id = log.id.to_string();
    let dto = AuditLogResponseDto::from(log);
    assert_eq!(dto.id, id);
    assert_eq!(dto.event_type, "ban");
    assert_eq!(dto.actor_id.as_deref(), Some("a"));
    assert!(dto.actor_name.is_none());
    assert_eq!(dto.target_name.as_deref(), Some("bob"));
    assert!(dto.created_at.contains('T'));
}

#[test]
fn query_params_all_optional() {
    let p: AuditLogQueryParams = serde_json::from_str("{}").unwrap();
    assert!(p.guild_id.is_none());
    assert!(p.event_type.is_none());
    assert!(p.actor_id.is_none());
    assert!(p.target_id.is_none());
    assert!(p.limit.is_none());
    assert!(p.offset.is_none());
}

#[test]
fn query_params_deserializes_all_fields() {
    let p: AuditLogQueryParams = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "event_type": "ban",
        "actor_id": "a", "target_id": "t",
        "limit": 50, "offset": 100
    }))
    .unwrap();
    assert_eq!(p.guild_id.as_deref(), Some("g"));
    assert_eq!(p.event_type.as_deref(), Some("ban"));
    assert_eq!(p.actor_id.as_deref(), Some("a"));
    assert_eq!(p.target_id.as_deref(), Some("t"));
    assert_eq!(p.limit, Some(50));
    assert_eq!(p.offset, Some(100));
}

#[test]
fn default_details_returns_empty_object() {
    assert_eq!(default_details(), serde_json::json!({}));
}

#[test]
fn create_dto_deserializes_with_custom_details() {
    let dto: CreateAuditLogDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "event_type": "x",
        "details": {"key": "value", "num": 42}
    }))
    .unwrap();
    assert_eq!(dto.details["key"], "value");
    assert_eq!(dto.details["num"], 42);
}

#[test]
fn create_dto_skips_all_optionals() {
    let dto: CreateAuditLogDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "event_type": "join"
    }))
    .unwrap();
    assert!(dto.actor_id.is_none());
    assert!(dto.actor_name.is_none());
    assert!(dto.target_id.is_none());
    assert!(dto.channel_id.is_none());
    assert_eq!(dto.details, serde_json::json!({}));
}

#[test]
fn response_dto_serializes_with_all_fields() {
    let now = Utc::now();
    let log = AuditLog {
        id: Uuid::new_v4(),
        guild_id: "guild-1".into(),
        event_type: "role_add".into(),
        actor_id: Some("mod-1".into()),
        actor_name: Some("ModOne".into()),
        target_id: Some("user-1".into()),
        target_name: Some("UserOne".into()),
        channel_id: Some("c-1".into()),
        channel_name: Some("general".into()),
        details: serde_json::json!({"role_id": "r1"}),
        created_at: now,
    };
    let dto: AuditLogResponseDto = log.into();
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"guild_id\":\"guild-1\""));
    assert!(json.contains("\"event_type\":\"role_add\""));
    assert!(json.contains("\"actor_name\":\"ModOne\""));
    assert!(json.contains("\"channel_name\":\"general\""));
    assert!(json.contains("\"role_id\":\"r1\""));
}

#[test]
fn response_dto_preserves_details_json_unchanged() {
    let log = AuditLog {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        event_type: "x".into(),
        actor_id: None,
        actor_name: None,
        target_id: None,
        target_name: None,
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({"array": [1, 2, 3], "nested": {"a": true}}),
        created_at: Utc::now(),
    };
    let dto: AuditLogResponseDto = log.into();
    assert_eq!(dto.details["array"][0], 1);
    assert_eq!(dto.details["nested"]["a"], true);
}
