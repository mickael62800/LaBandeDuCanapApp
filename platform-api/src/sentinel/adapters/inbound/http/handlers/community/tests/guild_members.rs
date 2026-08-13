use super::*;

// ── UpdateMemberPayload ──

#[test]
fn update_member_payload_all_optional() {
    let p: UpdateMemberPayload = serde_json::from_str(r#"{}"#).unwrap();
    assert!(p.username.is_none());
    assert!(p.display_name.is_none());
    assert!(p.avatar.is_none());
    assert!(p.roles.is_none());
}

#[test]
fn update_member_payload_partial() {
    let raw = r#"{"username":"alice_v2"}"#;
    let p: UpdateMemberPayload = serde_json::from_str(raw).unwrap();
    assert_eq!(p.username.as_deref(), Some("alice_v2"));
    assert!(p.display_name.is_none());
}

#[test]
fn update_member_payload_full() {
    let raw = r#"{
        "username":"alice","display_name":"Alice",
        "avatar":"hash","roles":["r1","r2"]
    }"#;
    let p: UpdateMemberPayload = serde_json::from_str(raw).unwrap();
    assert_eq!(p.avatar.as_deref(), Some("hash"));
    assert!(p.roles.as_ref().unwrap().is_array());
    assert_eq!(p.roles.unwrap().as_array().unwrap().len(), 2);
}

// ── SyncMembersPayload ──

#[test]
fn sync_members_payload_empty_array() {
    let raw = r#"{"guild_id":"g","members":[]}"#;
    let p: SyncMembersPayload = serde_json::from_str(raw).unwrap();
    assert_eq!(p.guild_id, "g".into());
    assert!(p.members.is_empty());
}
