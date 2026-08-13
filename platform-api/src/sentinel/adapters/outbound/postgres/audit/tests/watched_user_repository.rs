use super::*;
use chrono::Utc;

fn row(warns: i64, mutes: i64, bans: i64) -> WatchedUserRow {
    WatchedUserRow {
        user_id: "u".into(),
        username: "alice".into(),
        guild_id: "g".into(),
        guild_name: "server".into(),
        total_warns: warns,
        total_mutes: mutes,
        total_bans: bans,
        last_incident_at: None,
        security_events_count: 0,
        first_seen_at: Utc::now(),
    }
}

#[test]
fn risk_level_none_is_low() {
    let w = WatchedUser::from(row(0, 0, 0));
    assert_eq!(w.risk_level, "low");
}

#[test]
fn risk_level_single_warn_is_low() {
    // total=1 < 2 → low
    let w = WatchedUser::from(row(1, 0, 0));
    assert_eq!(w.risk_level, "low");
}

#[test]
fn risk_level_two_warns_is_medium() {
    // total=2, ni mute ni ban → medium
    let w = WatchedUser::from(row(2, 0, 0));
    assert_eq!(w.risk_level, "medium");
}

#[test]
fn risk_level_single_mute_is_high() {
    // 1 mute → high (peu importe le total)
    let w = WatchedUser::from(row(0, 1, 0));
    assert_eq!(w.risk_level, "high");
}

#[test]
fn risk_level_three_warns_without_mute_is_high() {
    // total >= 3 sans mute → high (via la branche total>=3)
    let w = WatchedUser::from(row(3, 0, 0));
    assert_eq!(w.risk_level, "high");
}

#[test]
fn risk_level_single_ban_is_critical() {
    // Un ban → critical, peu importe le reste
    let w = WatchedUser::from(row(0, 0, 1));
    assert_eq!(w.risk_level, "critical");
}

#[test]
fn risk_level_five_warns_is_critical() {
    // total >= 5 → critical même sans ban
    let w = WatchedUser::from(row(5, 0, 0));
    assert_eq!(w.risk_level, "critical");
}

#[test]
fn risk_level_ban_takes_priority_over_warns() {
    // 10 warns + 1 ban → critical (pas high malgre mute probable)
    let w = WatchedUser::from(row(10, 5, 1));
    assert_eq!(w.risk_level, "critical");
}

#[test]
fn risk_level_escalation_boundaries() {
    // Verifie les seuils exacts : 2/3/5.
    assert_eq!(WatchedUser::from(row(1, 0, 0)).risk_level, "low");
    assert_eq!(WatchedUser::from(row(2, 0, 0)).risk_level, "medium");
    assert_eq!(WatchedUser::from(row(3, 0, 0)).risk_level, "high");
    assert_eq!(WatchedUser::from(row(4, 0, 0)).risk_level, "high");
    assert_eq!(WatchedUser::from(row(5, 0, 0)).risk_level, "critical");
}

#[test]
fn risk_level_mute_upgrades_low_to_high() {
    // total=1 (1 mute), devrait etre "high" via la branche mute>0
    let w = WatchedUser::from(row(0, 1, 0));
    assert_eq!(w.risk_level, "high");
}

#[test]
fn fields_are_copied_unchanged() {
    let r = row(3, 1, 0);
    let w: WatchedUser = r.into();
    assert_eq!(w.user_id, "u".into());
    assert_eq!(w.username, "alice");
    assert_eq!(w.total_warns, 3);
    assert_eq!(w.total_mutes, 1);
    assert_eq!(w.total_bans, 0);
}
