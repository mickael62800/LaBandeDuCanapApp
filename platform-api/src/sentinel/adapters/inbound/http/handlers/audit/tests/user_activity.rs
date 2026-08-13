use super::*;

#[test]
fn default_metadata_returns_empty_json_object() {
    let m = default_metadata();
    assert!(m.is_object());
    assert_eq!(m.as_object().unwrap().len(), 0);
}

#[test]
fn create_activity_dto_deserialize_with_all_fields() {
    let raw = r#"{
        "guild_id": "g1",
        "user_id": "u1",
        "event_type": "message",
        "channel_id": "c1",
        "channel_name": "general",
        "content": "hello world",
        "metadata": {"foo": "bar"}
    }"#;
    let dto: CreateActivityDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g1".into());
    assert_eq!(dto.event_type, "message");
    assert_eq!(dto.channel_id.as_deref(), Some("c1"));
    assert_eq!(dto.content.as_deref(), Some("hello world"));
    assert_eq!(dto.metadata["foo"], "bar");
}

#[test]
fn create_activity_dto_deserialize_missing_metadata_uses_default() {
    let raw = r#"{
        "guild_id": "g",
        "user_id": "u",
        "event_type": "voice_join"
    }"#;
    let dto: CreateActivityDto = serde_json::from_str(raw).unwrap();
    // metadata absent → default_metadata = {}
    assert!(dto.metadata.is_object());
    assert_eq!(dto.metadata.as_object().unwrap().len(), 0);
    assert!(dto.channel_id.is_none());
    assert!(dto.content.is_none());
}

#[test]
fn activity_query_deserialize_all_optional() {
    // Tous les champs sont Option → JSON vide OK.
    let q: ActivityQuery = serde_json::from_str("{}").unwrap();
    assert!(q.event_type.is_none());
    assert!(q.limit.is_none());
    assert!(q.offset.is_none());
}

#[test]
fn activity_query_deserialize_with_filters() {
    let raw = r#"{"event_type":"voice_join","limit":25,"offset":10}"#;
    let q: ActivityQuery = serde_json::from_str(raw).unwrap();
    assert_eq!(q.event_type.as_deref(), Some("voice_join"));
    assert_eq!(q.limit, Some(25));
    assert_eq!(q.offset, Some(10));
}
