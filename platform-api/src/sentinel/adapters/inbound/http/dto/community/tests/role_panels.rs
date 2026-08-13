use super::*;
use chrono::TimeZone;
use chrono::Utc;
use platform_core::sentinel::domain::entities::community::role_panel::AutoRole;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanel;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelEntry;
use uuid::Uuid;

fn sample_panel() -> RolePanel {
    RolePanel {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        message_id: Some("m".into()),
        title: "Roles".into(),
        description: "pick".into(),
        mode: "button".into(),
        max_roles: Some(3),
        enabled: true,
        created_at: Utc.with_ymd_and_hms(2024, 3, 1, 0, 0, 0).unwrap(),
        updated_at: Utc::now(),
    }
}

fn sample_entry(panel_id: Uuid, pos: i32) -> RolePanelEntry {
    RolePanelEntry {
        id: Uuid::new_v4(),
        panel_id,
        role_id: "r1".into(),
        role_name: "Gamer".into(),
        emoji: Some("🎮".into()),
        label: "Gamer".into(),
        style: "primary".into(),
        position: pos,
    }
}

#[test]
fn default_mode_is_button() {
    let dto: CreateRolePanelDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "channel_id": "c", "title": "t", "entries": []
    }))
    .unwrap();
    assert_eq!(dto.mode, "button");
    assert_eq!(dto.description, "");
    assert!(dto.max_roles.is_none());
}

#[test]
fn default_entry_style_is_primary() {
    let dto: CreateEntryDto = serde_json::from_value(serde_json::json!({
        "role_id": "r", "role_name": "n"
    }))
    .unwrap();
    assert_eq!(dto.style, "primary");
    assert_eq!(dto.label, "");
    assert_eq!(dto.position, 0);
    assert!(dto.emoji.is_none());
}

#[test]
fn create_role_panel_dto_to_command_maps_entries() {
    let dto = CreateRolePanelDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        title: "t".into(),
        description: "d".into(),
        mode: "dropdown".into(),
        max_roles: Some(5),
        entries: vec![
            CreateEntryDto {
                role_id: "r1".into(),
                role_name: "R1".into(),
                emoji: None,
                label: "L".into(),
                style: "success".into(),
                position: 1,
            },
            CreateEntryDto {
                role_id: "r2".into(),
                role_name: "R2".into(),
                emoji: Some("x".into()),
                label: "".into(),
                style: "primary".into(),
                position: 2,
            },
        ],
    };
    let cmd: CreateRolePanelCommand = dto.into();
    assert_eq!(cmd.mode, "dropdown");
    assert_eq!(cmd.max_roles, Some(5));
    assert_eq!(cmd.entries.len(), 2);
    assert_eq!(cmd.entries[0].style, "success");
    assert_eq!(cmd.entries[1].emoji.as_deref(), Some("x"));
    assert_eq!(cmd.entries[1].position, 2);
}

#[test]
fn set_message_id_dto_to_command() {
    let dto = SetMessageIdDto {
        panel_id: "p".into(),
        message_id: "m".into(),
    };
    let cmd: SetMessageIdCommand = dto.into();
    assert_eq!(cmd.panel_id, "p");
    assert_eq!(cmd.message_id, "m".into());
}

#[test]
fn create_auto_role_dto_to_command_default_delay() {
    let dto: CreateAutoRoleDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "role_id": "r", "role_name": "n"
    }))
    .unwrap();
    assert_eq!(dto.delay_secs, 0);
    let cmd: CreateAutoRoleCommand = dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.delay_secs, 0);
}

#[test]
fn from_role_panel_preserves_fields_and_formats_date() {
    let panel = sample_panel();
    let id = panel.id.to_string();
    let dto = RolePanelDto::from(panel);
    assert_eq!(dto.id, id);
    assert_eq!(dto.message_id.as_deref(), Some("m"));
    assert_eq!(dto.max_roles, Some(3));
    assert!(dto.enabled);
    assert!(dto.created_at.starts_with("2024-03-01T"));
}

#[test]
fn from_role_panel_entry_preserves_fields() {
    let pid = Uuid::new_v4();
    let entry = sample_entry(pid, 7);
    let id = entry.id.to_string();
    let dto = RolePanelEntryDto::from(entry);
    assert_eq!(dto.id, id);
    assert_eq!(dto.role_id, "r1".into());
    assert_eq!(dto.position, 7);
    assert_eq!(dto.emoji.as_deref(), Some("🎮"));
}

#[test]
fn from_role_panel_detail_aggregates() {
    let panel = sample_panel();
    let pid = panel.id;
    let detail = RolePanelDetail {
        panel,
        entries: vec![sample_entry(pid, 0), sample_entry(pid, 1)],
    };
    let dto = RolePanelDetailDto::from(detail);
    assert_eq!(dto.entries.len(), 2);
    assert_eq!(dto.entries[1].position, 1);
    assert_eq!(dto.panel.guild_id, "g".into());
}

#[test]
fn from_auto_role_preserves_fields() {
    let a = AutoRole {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        role_id: "r".into(),
        role_name: "Member".into(),
        delay_secs: 30,
        enabled: false,
    };
    let id = a.id.to_string();
    let dto = AutoRoleDto::from(a);
    assert_eq!(dto.id, id);
    assert_eq!(dto.delay_secs, 30);
    assert!(!dto.enabled);
}
