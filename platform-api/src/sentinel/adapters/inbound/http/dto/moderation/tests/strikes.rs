use super::*;

fn base_add_dto(infraction_id: Option<String>) -> AddStrikeDto {
    AddStrikeDto {
        guild_id: "g".into(),
        user_id: "u".into(),
        reason: "spam".into(),
        source: "automod".into(),
        infraction_id,
    }
}

#[test]
fn add_strike_valid_infraction_id_parsed() {
    let id = "550e8400-e29b-41d4-a716-446655440000";
    let cmd: AddStrikeCommand = base_add_dto(Some(id.into())).into();
    assert_eq!(cmd.infraction_id.unwrap().to_string(), id);
}

#[test]
fn add_strike_invalid_uuid_becomes_none() {
    let cmd: AddStrikeCommand = base_add_dto(Some("not-a-uuid".into())).into();
    assert!(cmd.infraction_id.is_none());
}

#[test]
fn add_strike_empty_uuid_becomes_none() {
    let cmd: AddStrikeCommand = base_add_dto(Some("".into())).into();
    assert!(cmd.infraction_id.is_none());
}

#[test]
fn add_strike_no_infraction_id_stays_none() {
    let cmd: AddStrikeCommand = base_add_dto(None).into();
    assert!(cmd.infraction_id.is_none());
}

#[test]
fn add_strike_copies_all_fields() {
    let cmd: AddStrikeCommand = base_add_dto(None).into();
    assert_eq!(cmd.guild_id, "g".into());
    assert_eq!(cmd.user_id, "u".into());
    assert_eq!(cmd.reason, "spam");
    assert_eq!(cmd.source, "automod");
}

#[test]
fn strike_threshold_roundtrip_domain_to_dto_to_domain() {
    let original = StrikeThreshold {
        strikes: 3,
        action: "mute".into(),
        duration: Some(600),
    };
    let dto: StrikeThresholdDto = original.clone().into();
    let back: StrikeThreshold = dto.into();
    assert_eq!(back.strikes, original.strikes);
    assert_eq!(back.action, original.action);
    assert_eq!(back.duration, original.duration);
}

#[test]
fn save_config_dto_into_command_preserves_thresholds() {
    let dto = SaveStrikeConfigDto {
        window_secs: 3600,
        thresholds: vec![
            StrikeThresholdDto {
                strikes: 1,
                action: "warn".into(),
                duration: None,
            },
            StrikeThresholdDto {
                strikes: 3,
                action: "mute".into(),
                duration: Some(600),
            },
            StrikeThresholdDto {
                strikes: 5,
                action: "ban".into(),
                duration: None,
            },
        ],
        enabled: true,
    };
    let cmd = dto.into_command("g1".into());
    assert_eq!(cmd.guild_id, "g1".into());
    assert_eq!(cmd.window_secs, 3600);
    assert_eq!(cmd.thresholds.len(), 3);
    assert_eq!(cmd.thresholds[0].strikes, 1);
    assert_eq!(cmd.thresholds[2].action, "ban");
    assert!(cmd.enabled);
}

#[test]
fn save_config_empty_thresholds_preserved() {
    let dto = SaveStrikeConfigDto {
        window_secs: 60,
        thresholds: vec![],
        enabled: false,
    };
    let cmd = dto.into_command("g".into());
    assert!(cmd.thresholds.is_empty());
    assert!(!cmd.enabled);
}

#[test]
fn strike_config_dto_from_domain() {
    let config = StrikeConfig {
        guild_id: "g".into(),
        window_secs: 7200,
        thresholds: vec![StrikeThreshold {
            strikes: 5,
            action: "ban".into(),
            duration: None,
        }],
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let dto: StrikeConfigDto = config.into();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.window_secs, 7200);
    assert_eq!(dto.thresholds.len(), 1);
    assert!(dto.enabled);
}
