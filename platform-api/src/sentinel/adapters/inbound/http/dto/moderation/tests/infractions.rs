use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;
use platform_core::sentinel::domain::entities::moderation::infraction::Infraction;
use platform_core::sentinel::domain::enums::moderation::action::Action;
use uuid::Uuid;

fn sample_infraction(action: Action, duration: Option<u64>) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "alice".into(),
        display_name: None,
        message_id: "m".into(),
        content: "hello".into(),
        flags: DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        },
        score: 0.75,
        action,
        reason: "bad".into(),
        duration,
        created_at: Utc::now(),
    }
}

#[test]
fn from_infraction_maps_basic_fields() {
    let inf = sample_infraction(Action::Warn, None);
    let id = inf.id.to_string();
    let dto = InfractionResponseDto::from(inf);
    assert_eq!(dto.id, id);
    assert_eq!(dto.action, "warn");
    assert_eq!(dto.username, "alice");
    assert_eq!(dto.score, 0.75);
    assert_eq!(dto.duration, None);
}

#[test]
fn from_infraction_with_duration() {
    let dto = InfractionResponseDto::from(sample_infraction(Action::Mute, Some(600)));
    assert_eq!(dto.duration, Some(600));
    assert_eq!(dto.action, "mute");
}

#[test]
fn from_infraction_created_at_rfc3339() {
    let dto = InfractionResponseDto::from(sample_infraction(Action::Ban, None));
    assert!(dto.created_at.contains('T'));
}

#[test]
fn query_params_all_optional() {
    let params: InfractionQueryParams = serde_json::from_str("{}").unwrap();
    assert!(params.user_id.is_none());
    assert!(params.action.is_none());
    assert!(params.limit.is_none());
    assert!(params.offset.is_none());
}

#[test]
fn query_params_deserializes_fields() {
    let params: InfractionQueryParams = serde_json::from_value(serde_json::json!({
        "user_id": "u1", "action": "ban", "limit": 10, "offset": 5
    }))
    .unwrap();
    assert_eq!(params.user_id.as_deref(), Some("u1"));
    assert_eq!(params.action.as_deref(), Some("ban"));
    assert_eq!(params.limit, Some(10));
    assert_eq!(params.offset, Some(5));
}
