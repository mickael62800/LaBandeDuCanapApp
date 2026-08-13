use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::moderation::user_note::UserNote;
use uuid::Uuid;

#[test]
fn default_category_is_general() {
    assert_eq!(default_category(), "general");
}

#[test]
fn add_note_dto_to_command_preserves_fields() {
    let dto = AddNoteDto {
        guild_id: "g".into(),
        user_id: "u".into(),
        author_id: "mod".into(),
        author_name: "Mod".into(),
        content: "some note".into(),
        category: "security".into(),
    };
    let cmd: AddNoteCommand = dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.user_id, "u".into());
    assert_eq!(cmd.content, "some note");
    assert_eq!(cmd.category, "security");
}

#[test]
fn user_note_to_dto_formats_dates_rfc3339() {
    let note = UserNote {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        author_id: "mod".into(),
        author_name: "Mod".into(),
        content: "hi".into(),
        category: "general".into(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let dto: UserNoteDto = note.into();
    assert!(dto.created_at.contains('T'));
    assert!(dto.updated_at.contains('T'));
    assert_eq!(dto.content, "hi");
    assert_eq!(dto.category, "general");
}

#[test]
fn add_note_dto_deserializes_with_default_category() {
    let dto: AddNoteDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "user_id": "u",
        "author_id": "m", "author_name": "Mod",
        "content": "note sans category"
    }))
    .unwrap();
    assert_eq!(dto.category, "general");
}

#[test]
fn add_note_dto_deserializes_with_custom_category() {
    let dto: AddNoteDto = serde_json::from_value(serde_json::json!({
        "guild_id": "g", "user_id": "u",
        "author_id": "m", "author_name": "Mod",
        "content": "security note",
        "category": "security"
    }))
    .unwrap();
    assert_eq!(dto.category, "security");
}

#[test]
fn add_note_dto_preserves_all_fields_into_command() {
    let dto = AddNoteDto {
        guild_id: "guild-42".into(),
        user_id: "user-99".into(),
        author_id: "mod-7".into(),
        author_name: "ModName".into(),
        content: "multiline\ncontent\nwith\nbreaks".into(),
        category: "watch".into(),
    };
    let cmd: AddNoteCommand = dto.into();
    assert_eq!(cmd.guild_id, "guild-42".into());
    assert_eq!(cmd.user_id, "user-99".into());
    assert_eq!(cmd.author_id, "mod-7");
    assert_eq!(cmd.content, "multiline\ncontent\nwith\nbreaks");
    assert_eq!(cmd.category, "watch");
}

#[test]
fn user_note_dto_serializes_with_all_fields() {
    let now = Utc::now();
    let note = UserNote {
        id: Uuid::new_v4(),
        guild_id: "g1".into(),
        user_id: "u1".into(),
        author_id: "a1".into(),
        author_name: "A1".into(),
        content: "test".into(),
        category: "security".into(),
        created_at: now,
        updated_at: now,
    };
    let dto: UserNoteDto = note.into();
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"guild_id\":\"g1\""));
    assert!(json.contains("\"user_id\":\"u1\""));
    assert!(json.contains("\"category\":\"security\""));
    assert!(json.contains("\"content\":\"test\""));
}
