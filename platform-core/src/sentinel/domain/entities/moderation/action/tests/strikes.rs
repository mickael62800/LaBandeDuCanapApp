use super::*;

#[test]
fn default_for_guild_sets_defaults() {
    let c = StrikeConfig::default_for_guild("g1");
    assert_eq!(c.guild_id.as_str(), "g1");
    assert_eq!(c.window_secs, 3600);
    assert!(c.thresholds.is_empty());
    assert!(c.enabled);
}

#[test]
fn default_for_guild_copies_guild_id() {
    let c = StrikeConfig::default_for_guild("my-server");
    assert_eq!(c.guild_id.as_str(), "my-server");
}

#[test]
fn default_created_at_matches_updated_at() {
    let c = StrikeConfig::default_for_guild("g");
    assert_eq!(c.created_at, c.updated_at);
}

// ── StrikeResult::should_trigger_escalation_broadcast ────────────────

fn sample_strike_result(
    escalation_action: Option<&str>,
    escalation_duration: Option<u64>,
) -> StrikeResult {
    let now = chrono::Utc::now();
    StrikeResult {
        strike: UserStrike {
            id: Uuid::new_v4(),
            guild_id: "g".into(),
            user_id: "u".into(),
            reason: "test".into(),
            source: "test".into(),
            infraction_id: None,
            expires_at: None,
            created_at: now,
        },
        active_count: 3,
        escalation_action: escalation_action.map(|s| s.into()),
        escalation_duration,
    }
}

#[test]
fn should_trigger_escalation_broadcast_true_when_escalation_action_present() {
    assert!(sample_strike_result(Some("mute"), Some(3600)).should_trigger_escalation_broadcast());
    assert!(sample_strike_result(Some("ban"), None).should_trigger_escalation_broadcast());
}

#[test]
fn should_trigger_escalation_broadcast_false_when_no_escalation() {
    assert!(!sample_strike_result(None, None).should_trigger_escalation_broadcast());
    // Duration sans action reste false (invariant : duration sans action n'a pas de sens).
    assert!(!sample_strike_result(None, Some(3600)).should_trigger_escalation_broadcast());
}
