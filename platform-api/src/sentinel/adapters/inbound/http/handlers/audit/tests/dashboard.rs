use super::*;

#[test]
fn toggle_payload_deserializes_true() {
    let p: TogglePayload = serde_json::from_str(r#"{"enabled":true}"#).unwrap();
    assert!(p.enabled);
}

#[test]
fn toggle_payload_deserializes_false() {
    let p: TogglePayload = serde_json::from_str(r#"{"enabled":false}"#).unwrap();
    assert!(!p.enabled);
}

#[test]
fn heartbeat_payload_deserializes() {
    let p: HeartbeatPayload = serde_json::from_str(r#"{"name":"moderator-bot"}"#).unwrap();
    assert_eq!(p.name, "moderator-bot");
}

#[test]
fn heartbeat_payload_empty_name_allowed() {
    let p: HeartbeatPayload = serde_json::from_str(r#"{"name":""}"#).unwrap();
    assert!(p.name.is_empty());
}
