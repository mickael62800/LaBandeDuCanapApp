use super::*;
use serde_json::json;

// ── Constructors ──

#[test]
fn new_creates_broadcaster_without_redis() {
    let b = EventBroadcaster::new();
    assert!(b.redis_client.is_none());
}

#[test]
fn default_equivalent_to_new() {
    let b = EventBroadcaster::default();
    assert!(b.redis_client.is_none());
}

// ── broadcast() sans redis : no-op silencieux ──

#[test]
fn broadcast_without_redis_is_noop() {
    let b = EventBroadcaster::new();
    // Ne doit pas panic meme sans redis_client configure.
    b.broadcast("test_event", json!({"foo": "bar"}));
    b.broadcast("another", json!(null));
    b.broadcast("empty_obj", json!({}));
}

#[test]
fn broadcast_accepts_any_json_value() {
    let b = EventBroadcaster::new();
    b.broadcast("arr", json!([1, 2, 3]));
    b.broadcast("str", json!("hello"));
    b.broadcast("num", json!(42));
    b.broadcast("bool", json!(true));
}

// ── WsEvent serialization ──

#[test]
fn ws_event_with_guild_id_includes_field() {
    let e = WsEvent {
        event: "infraction".into(),
        guild_id: Some("g1".into()),
        data: json!({"id": 42}),
    };
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains("\"event\":\"infraction\""));
    assert!(json.contains("\"guild_id\":\"g1\""));
    assert!(json.contains("\"data\":{\"id\":42}"));
}

#[test]
fn ws_event_without_guild_id_omits_field() {
    let e = WsEvent {
        event: "system".into(),
        guild_id: None,
        data: json!({}),
    };
    let json = serde_json::to_string(&e).unwrap();
    assert!(
        !json.contains("guild_id"),
        "None doit etre omis grace a skip_serializing_if"
    );
    assert!(json.contains("\"event\":\"system\""));
}

// ── guild_id extraction depuis le payload ──

#[test]
fn broadcast_extracts_guild_id_from_payload() {
    // On ne peut pas observer directement l'extraction (pas de redis mock),
    // mais on s'assure que les payloads avec/sans guild_id ne paniquent pas.
    let b = EventBroadcaster::new();
    b.broadcast("with_gid", json!({"guild_id": "g1", "user_id": "u1"}));
    b.broadcast("without_gid", json!({"user_id": "u1"}));
    b.broadcast("non_string_gid", json!({"guild_id": 12345}));
}

#[test]
fn broadcast_with_complex_nested_payload() {
    let b = EventBroadcaster::new();
    b.broadcast(
        "complex",
        json!({
            "guild_id": "g1",
            "nested": {
                "field": [1, 2, {"inner": true}]
            }
        }),
    );
}
