use super::*;

#[test]
fn roundtrip_all_variants() {
    for s in ModerationActionType::VALID_VALUES {
        let action = ModerationActionType::from_str(s).unwrap();
        assert_eq!(action.as_str(), *s);
    }
}

#[test]
fn from_str_invalid() {
    assert!(ModerationActionType::from_str("kick").is_none());
    assert!(ModerationActionType::from_str("").is_none());
}

#[test]
fn is_ban() {
    assert!(ModerationActionType::BanTemp.is_ban());
    assert!(ModerationActionType::BanPermanent.is_ban());
    assert!(!ModerationActionType::Warn.is_ban());
    assert!(!ModerationActionType::MuteTemp.is_ban());
}

#[test]
fn is_mute() {
    assert!(ModerationActionType::MuteTemp.is_mute());
    assert!(ModerationActionType::MutePermanent.is_mute());
    assert!(!ModerationActionType::Warn.is_mute());
    assert!(!ModerationActionType::BanTemp.is_mute());
}

#[test]
fn serde_roundtrip() {
    let json = serde_json::to_string(&ModerationActionType::BanPermanent).unwrap();
    assert_eq!(json, "\"ban_permanent\"");
    let back: ModerationActionType = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ModerationActionType::BanPermanent);
}

#[test]
fn valid_values_count() {
    assert_eq!(ModerationActionType::VALID_VALUES.len(), 8);
}

#[test]
fn display_trait_uses_as_str() {
    // Couvre l'impl fmt::Display.
    for s in ModerationActionType::VALID_VALUES {
        let action = ModerationActionType::from_str(s).unwrap();
        assert_eq!(format!("{}", action), *s);
    }
}

#[test]
fn is_temporary_enum() {
    assert!(ModerationActionType::MuteTemp.is_temporary());
    assert!(ModerationActionType::BanTemp.is_temporary());
    assert!(!ModerationActionType::MutePermanent.is_temporary());
    assert!(!ModerationActionType::BanPermanent.is_temporary());
    assert!(!ModerationActionType::Warn.is_temporary());
    assert!(!ModerationActionType::Unmute.is_temporary());
    assert!(!ModerationActionType::Unban.is_temporary());
    assert!(!ModerationActionType::Call.is_temporary());
}

#[test]
fn is_temporary_str_matches_enum() {
    assert!(ModerationActionType::is_temporary_str("mute_temp"));
    assert!(ModerationActionType::is_temporary_str("ban_temp"));
    assert!(!ModerationActionType::is_temporary_str("mute_permanent"));
    assert!(!ModerationActionType::is_temporary_str("ban_permanent"));
    assert!(!ModerationActionType::is_temporary_str("warn"));
    assert!(!ModerationActionType::is_temporary_str(""));
    assert!(!ModerationActionType::is_temporary_str("unknown"));
    assert!(!ModerationActionType::is_temporary_str("MUTE_TEMP")); // case-sensitive
}
