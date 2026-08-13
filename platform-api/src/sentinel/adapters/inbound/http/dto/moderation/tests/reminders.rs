use super::*;

fn make_dto(action_id: &str) -> CreateReminderDto {
    CreateReminderDto {
        guild_id: "g".into(),
        moderator_id: "m".into(),
        moderator_name: "Mod".into(),
        target_id: "t".into(),
        target_name: "T".into(),
        action_type: "warn".into(),
        reason: "r".into(),
        action_id: action_id.into(),
        duration_secs: 86400,
        remind_before_secs: 3600,
    }
}

#[test]
fn default_remind_before_is_one_hour() {
    assert_eq!(default_remind_before(), 3600);
}

#[test]
fn from_dto_valid_uuid_preserved() {
    let id = "550e8400-e29b-41d4-a716-446655440000";
    let cmd: platform_core::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand =
        make_dto(id).into();
    assert_eq!(cmd.action_id.to_string(), id);
}

#[test]
fn from_dto_invalid_uuid_falls_back_to_nil() {
    let cmd: platform_core::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand =
        make_dto("not-a-uuid").into();
    assert_eq!(cmd.action_id, Uuid::nil());
}

#[test]
fn from_dto_empty_uuid_falls_back_to_nil() {
    let cmd: platform_core::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand =
        make_dto("").into();
    assert_eq!(cmd.action_id, Uuid::nil());
}

#[test]
fn from_dto_preserves_all_fields() {
    let dto = make_dto("550e8400-e29b-41d4-a716-446655440000");
    let cmd: platform_core::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand =
        dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.moderator_id, "m");
    assert_eq!(cmd.target_id, "t");
    assert_eq!(cmd.action_type, "warn");
    assert_eq!(cmd.reason, "r");
    assert_eq!(cmd.duration_secs, 86400);
    assert_eq!(cmd.remind_before_secs, 3600);
}
