use super::*;
use chrono::TimeZone;
use chrono::Utc;
fn ts() -> chrono::DateTime<chrono::Utc> {
    Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_role() -> DiscordRole {
    DiscordRole {
        id: "role-123".into(),
        guild_id: "g".into(),
        name: "Admin".into(),
        color: 0xFF0000,
        position: 5,
        permissions: 8,
        mentionable: true,
        managed: false,
        icon: Some("hash".into()),
        member_count: 42,
        synced_at: ts(),
    }
}

// ── DiscordRoleDto::from ──

#[test]
fn role_dto_serializes_permissions_as_string() {
    let dto = DiscordRoleDto::from(sample_role());
    // permissions i64 -> String pour eviter depasser MAX_SAFE_INTEGER cote JS
    assert_eq!(dto.permissions, "8");
}

#[test]
fn role_dto_handles_huge_permissions_bitfield() {
    let mut r = sample_role();
    r.permissions = 9_007_199_254_740_993; // > MAX_SAFE_INTEGER JS
    let dto = DiscordRoleDto::from(r);
    assert_eq!(dto.permissions, "9007199254740993");
}

#[test]
fn role_dto_preserves_all_scalar_fields() {
    let dto = DiscordRoleDto::from(sample_role());
    assert_eq!(dto.id, "role-123");
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.name, "Admin");
    assert_eq!(dto.color, 0xFF0000);
    assert_eq!(dto.position, 5);
    assert!(dto.mentionable);
    assert!(!dto.managed);
    assert_eq!(dto.icon.as_deref(), Some("hash"));
    assert_eq!(dto.member_count, 42);
    assert_eq!(dto.synced_at, ts().to_rfc3339());
}

#[test]
fn role_dto_none_icon_preserved() {
    let mut r = sample_role();
    r.icon = None;
    let dto = DiscordRoleDto::from(r);
    assert!(dto.icon.is_none());
}

// ── CreateRoleRequest + EditRoleRequest ──

#[test]
fn create_role_request_minimal() {
    let raw = r#"{"name":"NewRole","color":255}"#;
    let req: CreateRoleRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.name, "NewRole");
    assert_eq!(req.color, 255);
    assert!(req.permissions.is_none());
}

#[test]
fn create_role_request_with_permissions() {
    let raw = r#"{"name":"Admin","color":0,"permissions":"8"}"#;
    let req: CreateRoleRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.permissions.as_deref(), Some("8"));
}

#[test]
fn edit_role_request_all_optional() {
    let req: EditRoleRequest = serde_json::from_str(r#"{}"#).unwrap();
    assert!(req.name.is_none());
    assert!(req.color.is_none());
    assert!(req.permissions.is_none());
    assert!(req.mentionable.is_none());
    assert!(req.hoist.is_none());
}

#[test]
fn edit_role_request_partial_update() {
    let raw = r#"{"name":"Renamed","mentionable":true}"#;
    let req: EditRoleRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.name.as_deref(), Some("Renamed"));
    assert_eq!(req.mentionable, Some(true));
    assert!(req.color.is_none());
}

#[test]
fn edit_role_request_full_update() {
    let raw = r#"{"name":"X","color":100,"permissions":"8","mentionable":false,"hoist":true}"#;
    let req: EditRoleRequest = serde_json::from_str(raw).unwrap();
    assert_eq!(req.color, Some(100));
    assert_eq!(req.hoist, Some(true));
}
