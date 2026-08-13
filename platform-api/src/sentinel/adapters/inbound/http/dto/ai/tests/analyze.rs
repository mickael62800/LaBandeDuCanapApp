use super::*;
use platform_core::sentinel::domain::entities::moderation::detection_flags::DetectionFlags;

fn make_dto(content: String, n_context: usize) -> AnalyzeRequestDto {
    let context_messages = (0..n_context)
        .map(|i| ContextMessageDto {
            username: format!("u{i}"),
            content: format!("msg{i}"),
        })
        .collect();
    AnalyzeRequestDto {
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "alice".into(),
        content,
        flags: DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        },
        metadata: MetadataDto {
            message_id: "m1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
        },
        context_messages,
    }
}

#[test]
fn from_dto_short_content_preserved() {
    let dto = make_dto("hello world".into(), 0);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert_eq!(cmd.content, "hello world");
}

#[test]
fn from_dto_content_exactly_2500_preserved() {
    let content = "a".repeat(2500);
    let dto = make_dto(content.clone(), 0);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert_eq!(cmd.content.len(), 2500);
    assert_eq!(cmd.content, content);
}

#[test]
fn from_dto_content_above_2500_truncated() {
    let content = "x".repeat(3000);
    let dto = make_dto(content, 0);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert_eq!(cmd.content.chars().count(), 2500);
    assert!(cmd.content.chars().all(|c| c == 'x'));
}

#[test]
fn from_dto_truncation_counts_chars_not_bytes() {
    // "é" est 2 bytes mais 1 char. 3000 "é" = 6000 bytes, 3000 chars.
    // Apres truncation : 2500 chars = 5000 bytes.
    let content: String = "é".repeat(3000);
    let dto = make_dto(content, 0);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert_eq!(cmd.content.chars().count(), 2500);
}

#[test]
fn from_dto_copies_ids_and_flags() {
    let dto = make_dto("hi".into(), 0);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.channel_id, "c".into());
    assert_eq!(cmd.user_id, "u".into());
    assert_eq!(cmd.username, "alice");
    assert_eq!(cmd.message_id, "m1".into());
    assert_eq!(cmd.timestamp, "2026-01-01T00:00:00Z");
}

#[test]
fn from_dto_maps_context_messages() {
    let dto = make_dto("hi".into(), 3);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert_eq!(cmd.context_messages.len(), 3);
    assert_eq!(cmd.context_messages[0].username, "u0");
    assert_eq!(cmd.context_messages[0].content, "msg0");
    assert_eq!(cmd.context_messages[2].username, "u2");
}

#[test]
fn from_dto_empty_context_produces_empty_vec() {
    let dto = make_dto("x".into(), 0);
    let cmd: platform_core::sentinel::ports::inbound::ai::analyze_message::AnalyzeMessageCommand =
        dto.into();
    assert!(cmd.context_messages.is_empty());
}

// ── AnalyzeResponseDto from MessageAnalysis ──

#[test]
fn response_dto_empty_reason_becomes_none() {
    use platform_core::sentinel::domain::entities::ai::message_analysis::MessageAnalysis;
    use platform_core::sentinel::domain::enums::moderation::action::Action;
    let analysis = MessageAnalysis {
        action: Action::Warn,
        reason: String::new(),
        duration: None,
        score: 0.0,
        route:
            platform_core::sentinel::domain::services::moderation::automod_routing::Routing::Card,
        auto_action: false,
        severe: false,
        auto_delete_link: false,
    };
    let dto: AnalyzeResponseDto = analysis.into();
    assert_eq!(dto.action, "warn");
    assert!(dto.reason.is_none());
}

#[test]
fn response_dto_non_empty_reason_is_some() {
    use platform_core::sentinel::domain::entities::ai::message_analysis::MessageAnalysis;
    use platform_core::sentinel::domain::enums::moderation::action::Action;
    let analysis = MessageAnalysis {
        action: Action::Ban,
        reason: "spam".into(),
        duration: Some(60),
        score: 8.5,
        route:
            platform_core::sentinel::domain::services::moderation::automod_routing::Routing::Card,
        auto_action: false,
        severe: false,
        auto_delete_link: false,
    };
    let dto: AnalyzeResponseDto = analysis.into();
    assert_eq!(dto.action, "ban");
    assert_eq!(dto.reason.unwrap(), "spam");
    assert_eq!(dto.duration, Some(60));
}
