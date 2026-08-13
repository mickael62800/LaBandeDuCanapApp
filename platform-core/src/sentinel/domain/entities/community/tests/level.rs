use super::*;

#[test]
fn test_xp_for_level() {
    assert_eq!(xp_for_level(0), 0);
    assert_eq!(xp_for_level(1), 155); // 5 + 50 + 100
    assert_eq!(xp_for_level(2), 220); // 20 + 100 + 100
}

#[test]
fn test_level_from_xp() {
    // Base 1 : 0 XP = niveau 1 (modele RPG), puis +1 a chaque palier franchi.
    assert_eq!(level_from_xp(0), 1);
    assert_eq!(level_from_xp(154), 1);
    assert_eq!(level_from_xp(155), 2);
    assert_eq!(level_from_xp(374), 2);
    assert_eq!(level_from_xp(375), 3);
}

#[test]
fn test_xp_progress() {
    let (current, needed) = xp_progress(200);
    assert_eq!(current, 200 - 155);
    assert_eq!(needed, 220);
}

#[test]
fn xp_source_as_str_all_variants() {
    assert_eq!(XpSource::Text.as_str(), "text");
    assert_eq!(XpSource::Voice.as_str(), "voice");
}

#[test]
fn xp_source_from_str_lossy() {
    assert_eq!(XpSource::from_str("text"), XpSource::Text);
    assert_eq!(XpSource::from_str("voice"), XpSource::Voice);
    // Unknown → Text (fallback).
    assert_eq!(XpSource::from_str(""), XpSource::Text);
    assert_eq!(XpSource::from_str("unknown"), XpSource::Text);
    assert_eq!(XpSource::from_str("TEXT"), XpSource::Text); // case-sensitive
}
