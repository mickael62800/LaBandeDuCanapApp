use super::*;

#[test]
fn as_str_all_variants() {
    assert_eq!(ModerationGravity::Low.as_str(), "low");
    assert_eq!(ModerationGravity::Medium.as_str(), "medium");
    assert_eq!(ModerationGravity::High.as_str(), "high");
    assert_eq!(ModerationGravity::Critical.as_str(), "critical");
}

#[test]
fn from_str_lossy_valid() {
    assert_eq!(
        ModerationGravity::from_str_lossy("low"),
        Some(ModerationGravity::Low)
    );
    assert_eq!(
        ModerationGravity::from_str_lossy("medium"),
        Some(ModerationGravity::Medium)
    );
    assert_eq!(
        ModerationGravity::from_str_lossy("high"),
        Some(ModerationGravity::High)
    );
    assert_eq!(
        ModerationGravity::from_str_lossy("critical"),
        Some(ModerationGravity::Critical)
    );
}

#[test]
fn from_str_lossy_invalid() {
    assert_eq!(ModerationGravity::from_str_lossy(""), None);
    assert_eq!(ModerationGravity::from_str_lossy("LOW"), None); // case-sensitive
    assert_eq!(ModerationGravity::from_str_lossy("urgent"), None);
    assert_eq!(ModerationGravity::from_str_lossy("unknown"), None);
}

#[test]
fn ordering_follows_severity() {
    // Invariant metier : Low < Medium < High < Critical.
    assert!(ModerationGravity::Low < ModerationGravity::Medium);
    assert!(ModerationGravity::Medium < ModerationGravity::High);
    assert!(ModerationGravity::High < ModerationGravity::Critical);
}

#[test]
fn roundtrip_via_as_str() {
    for g in [
        ModerationGravity::Low,
        ModerationGravity::Medium,
        ModerationGravity::High,
        ModerationGravity::Critical,
    ] {
        assert_eq!(ModerationGravity::from_str_lossy(g.as_str()), Some(g));
    }
}

#[test]
fn serde_lowercase() {
    let json = serde_json::to_string(&ModerationGravity::High).unwrap();
    assert_eq!(json, "\"high\"");
    let back: ModerationGravity = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ModerationGravity::High);
}
