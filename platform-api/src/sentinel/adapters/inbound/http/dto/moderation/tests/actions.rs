use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::moderation::action::applied::ModerationAction;
use platform_core::sentinel::domain::entities::moderation::action::applied::UserModerationHistory;
use platform_core::sentinel::domain::enums::moderation::moderation_gravity::ModerationGravity;
use uuid::Uuid;

fn sample_action() -> ModerationAction {
    ModerationAction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Alice".into(),
        target_display_name: None,
        action_type: "ban_temp".into(),
        reason: "spam".into(),
        gravity: Some(ModerationGravity::High),
        duration: Some(3600),
        created_at: Utc::now(),
    }
}

#[test]
fn log_action_dto_to_command_preserves_fields() {
    let dto = LogActionDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "Mod".into(),
        target_id: "u".into(),
        target_name: "Alice".into(),
        action_type: "warn".into(),
        reason: "test".into(),
        gravity: Some("medium".into()),
        duration: Some(600),
    };
    let cmd: LogModerationCommand = dto.into();
    assert_eq!(cmd.action_type, "warn");
    assert_eq!(cmd.gravity, Some("medium".into()));
    assert_eq!(cmd.duration, Some(600));
}

#[test]
fn log_action_dto_optional_fields_none() {
    let dto = LogActionDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "mod".into(),
        moderator_name: "M".into(),
        target_id: "u".into(),
        target_name: "U".into(),
        action_type: "warn".into(),
        reason: "x".into(),
        gravity: None,
        duration: None,
    };
    let cmd: LogModerationCommand = dto.into();
    assert!(cmd.gravity.is_none());
    assert!(cmd.duration.is_none());
}

#[test]
fn action_to_response_dto_strips_metadata() {
    let a = sample_action();
    let dto: ModerationActionResponseDto = a.into();
    // Metadata d'escalation est None par defaut (pas mappee directement).
    assert!(dto.escalation_action.is_none());
    assert!(dto.escalation_duration.is_none());
    assert!(dto.strikes_count.is_none());
    assert_eq!(dto.action_type, "ban_temp");
    assert_eq!(dto.target_name, "Alice");
}

#[test]
fn action_to_ban_entry_dto_copies_ids() {
    let a = sample_action();
    let id = a.id;
    let dto: BanEntryDto = a.into();
    assert_eq!(dto.id, id.to_string());
    assert_eq!(dto.action_type, "ban_temp");
    assert!(dto.created_at.contains('T'));
}

#[test]
fn user_history_to_dto_aggregates() {
    let history = UserModerationHistory {
        target_id: "u".into(),
        target_name: "Alice".into(),
        total_warns: 3,
        total_mutes: 1,
        total_bans: 0,
        actions: vec![sample_action(), sample_action()],
    };
    let dto: UserHistoryDto = history.into();
    assert_eq!(dto.total_warns, 3);
    assert_eq!(dto.actions.len(), 2);
}

#[test]
fn log_action_dto_deserializes_from_json() {
    let dto: LogActionDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "channel_id": "c",
        "moderator_id": "mod", "moderator_name": "Mod",
        "target_id": "t", "target_name": "T",
        "action_type": "mute", "reason": "spam",
        "gravity": "high", "duration": 1800
    }))
    .unwrap();
    assert_eq!(dto.action_type, "mute");
    assert_eq!(dto.gravity.as_deref(), Some("high"));
    assert_eq!(dto.duration, Some(1800));
}

#[test]
fn log_action_dto_deserializes_without_optionals() {
    let dto: LogActionDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "channel_id": "c",
        "moderator_id": "m", "moderator_name": "M",
        "target_id": "t", "target_name": "T",
        "action_type": "warn", "reason": "r"
    }))
    .unwrap();
    assert!(dto.gravity.is_none());
    assert!(dto.duration.is_none());
}

#[test]
fn moderation_action_response_dto_serializes_skipping_none_escalation() {
    let dto = ModerationActionResponseDto {
        id: "id-1".into(),
        action_type: "warn".into(),
        target_name: "Alice".into(),
        reason: "test".into(),
        escalation_action: None,
        escalation_duration: None,
        strikes_count: None,
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"id\":\"id-1\""));
    // skip_serializing_if skips optional None fields
    assert!(!json.contains("escalation_action"));
    assert!(!json.contains("escalation_duration"));
    assert!(!json.contains("strikes_count"));
}

#[test]
fn moderation_action_response_dto_with_escalation_serializes_fields() {
    let dto = ModerationActionResponseDto {
        id: "id-1".into(),
        action_type: "warn".into(),
        target_name: "Alice".into(),
        reason: "test".into(),
        escalation_action: Some("ban".into()),
        escalation_duration: Some(7200),
        strikes_count: Some(3),
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"escalation_action\":\"ban\""));
    assert!(json.contains("\"escalation_duration\":7200"));
    assert!(json.contains("\"strikes_count\":3"));
}

#[test]
fn ban_entry_dto_serializes_rfc3339_date() {
    let a = sample_action();
    let dto: BanEntryDto = a.into();
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"created_at\":"));
    assert!(json.contains("T"));
}

#[test]
fn mod_stats_entry_dto_serializes() {
    let dto = ModStatsEntryDto {
        moderator_id: "m1".into(),
        moderator_name: "ModOne".into(),
        total: 42,
        warns: 20,
        mutes: 10,
        bans: 5,
        kicks: 7,
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"moderator_id\":\"m1\""));
    assert!(json.contains("\"total\":42"));
    assert!(json.contains("\"warns\":20"));
    assert!(json.contains("\"kicks\":7"));
}

#[test]
fn user_history_empty_actions() {
    let history = UserModerationHistory {
        target_id: "u".into(),
        target_name: "X".into(),
        total_warns: 0,
        total_mutes: 0,
        total_bans: 0,
        actions: vec![],
    };
    let dto: UserHistoryDto = history.into();
    assert!(dto.actions.is_empty());
}

#[test]
fn action_to_response_dto_preserves_target_name() {
    let mut a = sample_action();
    a.target_name = "Bob The Slayer".into();
    let dto: ModerationActionResponseDto = a.into();
    assert_eq!(dto.target_name, "Bob The Slayer");
}

#[test]
fn log_action_dto_long_reason_truncation_is_service_side() {
    // Le DTO ne fait pas la truncation : il passe le texte tel quel.
    // Le service ManageModerationService truncate a 500 chars.
    let long = "x".repeat(1000);
    let dto = LogActionDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        moderator_id: "m".into(),
        moderator_name: "M".into(),
        target_id: "t".into(),
        target_name: "T".into(),
        action_type: "warn".into(),
        reason: long.clone(),
        gravity: None,
        duration: None,
    };
    let cmd: LogModerationCommand = dto.into();
    assert_eq!(cmd.reason.len(), 1000);
}
