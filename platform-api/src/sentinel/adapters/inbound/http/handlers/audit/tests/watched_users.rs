use super::*;

// ── WatchedUsersQueryParams ──

#[test]
fn watched_users_query_all_optional() {
    let q: WatchedUsersQueryParams = serde_json::from_str(r#"{}"#).unwrap();
    assert!(q.guild_id.is_none());
    assert!(q.limit.is_none());
    assert!(q.offset.is_none());
}

#[test]
fn watched_users_query_full() {
    let raw = r#"{"guild_id":"g","limit":50,"offset":10}"#;
    let q: WatchedUsersQueryParams = serde_json::from_str(raw).unwrap();
    assert_eq!(q.guild_id.as_deref(), Some("g"));
    assert_eq!(q.limit, Some(50));
    assert_eq!(q.offset, Some(10));
}

// ── AddWatchDto ──

#[test]
fn add_watch_dto_default_reason_empty() {
    let raw = r#"{"guild_id":"g","user_id":"u","username":"Alice"}"#;
    let dto: AddWatchDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.user_id, "u".into());
    assert_eq!(dto.username, "Alice");
    assert!(dto.reason.is_empty());
}

#[test]
fn add_watch_dto_with_reason() {
    let raw = r#"{"guild_id":"g","user_id":"u","username":"Alice","reason":"spam suspect"}"#;
    let dto: AddWatchDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.reason, "spam suspect");
}
