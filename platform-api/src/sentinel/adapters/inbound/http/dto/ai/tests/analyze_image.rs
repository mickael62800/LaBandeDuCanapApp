use super::*;
use platform_core::sentinel::domain::entities::ai::image_analysis::ImageAnalysis;
use platform_core::sentinel::domain::entities::ai::image_analysis::ImageClassification;
use platform_core::sentinel::domain::enums::moderation::action::Action;

fn sample_analysis(action: Action, reason: &str, duration: Option<u64>) -> ImageAnalysis {
    ImageAnalysis {
        action,
        reason: reason.into(),
        score: 0.9,
        duration,
        classifications: vec![
            ImageClassification {
                label: "nsfw".into(),
                confidence: 0.87,
            },
            ImageClassification {
                label: "safe".into(),
                confidence: 0.13,
            },
        ],
    }
}

#[test]
fn response_from_analysis_maps_action_and_classifications() {
    let dto: AnalyzeImageResponseDto =
        sample_analysis(Action::Delete, "nsfw detected", None).into();
    assert_eq!(dto.action, "delete");
    assert_eq!(dto.reason.as_deref(), Some("nsfw detected"));
    assert_eq!(dto.duration, None);
    assert_eq!(dto.classifications.len(), 2);
    assert_eq!(dto.classifications[0].label, "nsfw");
    assert!((dto.classifications[0].confidence - 0.87).abs() < 1e-6);
}

#[test]
fn response_empty_reason_becomes_none() {
    let dto: AnalyzeImageResponseDto = sample_analysis(Action::None, "", None).into();
    assert_eq!(dto.reason, None);
}

#[test]
fn response_preserves_duration() {
    let dto: AnalyzeImageResponseDto = sample_analysis(Action::Mute, "x", Some(3600)).into();
    assert_eq!(dto.duration, Some(3600));
}

#[test]
fn response_skips_none_fields_in_json() {
    let dto: AnalyzeImageResponseDto = sample_analysis(Action::None, "", None).into();
    let json = serde_json::to_value(&dto).unwrap();
    assert!(json.get("reason").is_none());
    assert!(json.get("duration").is_none());
    assert_eq!(json["action"], "none");
}

#[test]
fn response_serializes_with_some_fields() {
    let dto: AnalyzeImageResponseDto = sample_analysis(Action::Ban, "abuse", Some(600)).into();
    let json = serde_json::to_value(&dto).unwrap();
    assert_eq!(json["reason"], "abuse");
    assert_eq!(json["duration"], 600);
    assert_eq!(json["action"], "ban");
}

#[test]
fn request_deserializes_all_fields() {
    let raw = serde_json::json!({
        "guild_id": "g",
        "channel_id": "c",
        "user_id": "u",
        "username": "alice",
        "message_id": "m",
        "image_data": "aGVsbG8=",
        "content_type": "image/png",
        "filename": "pic.png"
    });
    let dto: AnalyzeImageRequestDto = serde_json::from_value(raw).unwrap();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.image_data, "aGVsbG8=");
    assert_eq!(dto.content_type, "image/png");
    assert_eq!(dto.filename, "pic.png");
}

#[test]
fn response_empty_classifications() {
    let mut a = sample_analysis(Action::None, "x", None);
    a.classifications.clear();
    let dto: AnalyzeImageResponseDto = a.into();
    assert!(dto.classifications.is_empty());
}
