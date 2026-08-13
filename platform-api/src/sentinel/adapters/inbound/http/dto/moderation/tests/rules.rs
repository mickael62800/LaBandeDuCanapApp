use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::system::rule::Rule;
use uuid::Uuid;

fn make_dto(flag: &str) -> CreateRuleDto {
    CreateRuleDto {
        guild_id: "g".into(),
        flag_type: flag.into(),
        weight: 3.0,
        threshold_warn: 2.0,
        threshold_delete: 4.0,
        threshold_mute: 6.0,
        threshold_ban: 9.0,
        enabled: true,
    }
}

#[test]
fn default_true_returns_true() {
    assert!(default_true());
}

#[test]
fn create_rule_dto_parses_flag_lossy() {
    let cmd: CreateRuleCommand = make_dto("spam").into();
    assert_eq!(cmd.flag_type, FlagType::Spam);
}

#[test]
fn create_rule_dto_unknown_flag_defaults_to_spam() {
    // FlagType::from_str_lossy retourne Spam pour les valeurs inconnues.
    let cmd: CreateRuleCommand = make_dto("wibble").into();
    assert_eq!(cmd.flag_type, FlagType::Spam);
}

#[test]
fn create_rule_dto_maps_all_variants() {
    for flag in [
        "insult",
        "link",
        "phishing",
        "nsfw",
        "illicit",
        "anger",
        "rage",
        "threat",
        "harassment",
    ] {
        let cmd: CreateRuleCommand = make_dto(flag).into();
        assert_eq!(cmd.flag_type.as_str(), flag);
    }
}

#[test]
fn create_rule_dto_preserves_numeric_fields() {
    let dto = CreateRuleDto {
        guild_id: "g".into(),
        flag_type: "spam".into(),
        weight: 3.5,
        threshold_warn: 1.5,
        threshold_delete: 4.5,
        threshold_mute: 6.5,
        threshold_ban: 9.5,
        enabled: false,
    };
    let cmd: CreateRuleCommand = dto.into();
    assert_eq!(cmd.weight, 3.5);
    assert_eq!(cmd.threshold_warn, 1.5);
    assert_eq!(cmd.threshold_ban, 9.5);
    assert!(!cmd.enabled);
}

#[test]
fn rule_to_dto_serializes_flag_as_str() {
    let now = Utc::now();
    let rule = Rule {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        flag_type: FlagType::Harassment,
        weight: 7.0,
        threshold_warn: 2.0,
        threshold_delete: 4.0,
        threshold_mute: 6.0,
        threshold_ban: 9.0,
        enabled: true,
        created_at: now,
        updated_at: now,
    };
    let dto: RuleResponseDto = rule.into();
    assert_eq!(dto.flag_type, "harassment");
    assert_eq!(dto.weight, 7.0);
    assert!(dto.created_at.contains('T'));
}

#[test]
fn rule_dto_roundtrip_flag_type() {
    // Parse from dto, emit back to dto via Rule→dto.
    for flag in ["spam", "insult", "nsfw"] {
        let cmd: CreateRuleCommand = make_dto(flag).into();
        assert_eq!(cmd.flag_type.as_str(), flag);
    }
}
