use super::*;

// ── LeaderboardQuery ──

#[test]
fn leaderboard_query_empty_limit_none() {
    let q: LeaderboardQuery = serde_json::from_str(r#"{}"#).unwrap();
    assert!(q.limit.is_none());
}

#[test]
fn leaderboard_query_with_limit() {
    let q: LeaderboardQuery = serde_json::from_str(r#"{"limit":20}"#).unwrap();
    assert_eq!(q.limit, Some(20));
}

// ── LogQuery ──

#[test]
fn log_query_empty_limit_none() {
    let q: LogQuery = serde_json::from_str(r#"{}"#).unwrap();
    assert!(q.limit.is_none());
}

#[test]
fn log_query_with_limit() {
    let q: LogQuery = serde_json::from_str(r#"{"limit":50}"#).unwrap();
    assert_eq!(q.limit, Some(50));
}
