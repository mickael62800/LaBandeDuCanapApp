use super::*;
use chrono::Utc;

fn row_with_flags(flags: serde_json::Value) -> InfractionRow {
    InfractionRow {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "alice".into(),
        display_name: None,
        message_id: "m".into(),
        content: "hello".into(),
        flags,
        score: 5.0,
        action: "warn".into(),
        reason: "test".into(),
        duration: None,
        created_at: Utc::now(),
    }
}

#[test]
fn infraction_from_row_valid_flags() {
    let json = serde_json::json!({
        "spam": true,
        "insult": false,
        "link": false,
        "phishing": true,
    });
    let infr = Infraction::from(row_with_flags(json));
    assert!(infr.flags.spam);
    assert!(!infr.flags.insult);
    assert!(infr.flags.phishing);
}

#[test]
fn infraction_from_row_invalid_flags_falls_back_to_default() {
    // Flags corrompu → fallback tout a false (sans log — pattern silencieux)
    let json = serde_json::json!("garbage");
    let infr = Infraction::from(row_with_flags(json));
    assert!(!infr.flags.spam);
    assert!(!infr.flags.insult);
    assert!(!infr.flags.link);
    assert!(!infr.flags.phishing);
}

#[test]
fn infraction_from_row_null_flags_falls_back() {
    let infr = Infraction::from(row_with_flags(serde_json::Value::Null));
    assert!(!infr.flags.spam);
    assert!(!infr.flags.phishing);
}

#[test]
fn infraction_from_row_phishing_default_false_if_missing() {
    // phishing est #[serde(default)] dans DetectionFlags → OK si absent
    let json = serde_json::json!({
        "spam": true, "insult": false, "link": false
    });
    let infr = Infraction::from(row_with_flags(json));
    assert!(infr.flags.spam);
    assert!(!infr.flags.phishing);
}

#[test]
fn infraction_action_parsed_lossy() {
    // action "warn" → Action::Warn, inconnu → Action::None
    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.action = "mute".into();
    let infr = Infraction::from(r);
    assert_eq!(infr.action, Action::Mute);

    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.action = "unknown_action".into();
    let infr = Infraction::from(r);
    assert_eq!(infr.action, Action::None);
}

#[test]
fn infraction_duration_some_i64_to_u64() {
    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.duration = Some(3600);
    let infr = Infraction::from(r);
    assert_eq!(infr.duration, Some(3600));
}

#[test]
fn infraction_duration_none_passthrough() {
    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.duration = None;
    let infr = Infraction::from(r);
    assert!(infr.duration.is_none());
}

#[test]
fn infraction_negative_duration_becomes_none() {
    // Regression : anciennement `duration as u64` wrap sur u64::MAX pour negatif.
    // Fix : `u64::try_from(d).ok()` → None pour negatif.
    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.duration = Some(-1);
    let infr = Infraction::from(r);
    assert!(infr.duration.is_none());

    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.duration = Some(i64::MIN);
    let infr = Infraction::from(r);
    assert!(infr.duration.is_none());
}

#[test]
fn infraction_zero_duration_kept() {
    // 0 est valide (action instantanee), pas confondu avec None.
    let mut r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    r.duration = Some(0);
    let infr = Infraction::from(r);
    assert_eq!(infr.duration, Some(0));
}

#[test]
fn infraction_preserves_content_and_score() {
    let r = row_with_flags(serde_json::json!({"spam":false,"insult":false,"link":false}));
    let infr = Infraction::from(r);
    assert_eq!(infr.content, "hello");
    assert_eq!(infr.score, 5.0);
    assert_eq!(infr.username, "alice");
}
