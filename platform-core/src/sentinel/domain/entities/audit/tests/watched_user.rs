use super::*;

#[test]
fn risk_level_none_is_low() {
    assert_eq!(classify_risk_level(0, 0, 0), "low");
}

#[test]
fn risk_level_single_warn_is_low() {
    // total=1 < 2 → low
    assert_eq!(classify_risk_level(1, 0, 0), "low");
}

#[test]
fn risk_level_two_warns_is_medium() {
    assert_eq!(classify_risk_level(2, 0, 0), "medium");
}

#[test]
fn risk_level_single_mute_is_high() {
    assert_eq!(classify_risk_level(0, 1, 0), "high");
}

#[test]
fn risk_level_three_warns_without_mute_is_high() {
    assert_eq!(classify_risk_level(3, 0, 0), "high");
}

#[test]
fn risk_level_single_ban_is_critical() {
    assert_eq!(classify_risk_level(0, 0, 1), "critical");
}

#[test]
fn risk_level_five_warns_is_critical() {
    assert_eq!(classify_risk_level(5, 0, 0), "critical");
}

#[test]
fn risk_level_ban_takes_priority_over_warns() {
    assert_eq!(classify_risk_level(10, 5, 1), "critical");
}

#[test]
fn risk_level_escalation_boundaries() {
    assert_eq!(classify_risk_level(1, 0, 0), "low");
    assert_eq!(classify_risk_level(2, 0, 0), "medium");
    assert_eq!(classify_risk_level(3, 0, 0), "high");
    assert_eq!(classify_risk_level(4, 0, 0), "high");
    assert_eq!(classify_risk_level(5, 0, 0), "critical");
}

#[test]
fn risk_level_mute_upgrades_low_to_high() {
    assert_eq!(classify_risk_level(0, 1, 0), "high");
}
