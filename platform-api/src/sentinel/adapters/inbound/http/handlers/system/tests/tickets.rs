use super::*;

#[test]
fn bulk_delete_tickets_params_all_optional() {
    let p: BulkDeleteTicketsParams = serde_json::from_str(r#"{}"#).unwrap();
    assert!(p.author_id.is_none());
    assert!(p.from.is_none());
    assert!(p.to.is_none());
    // all a #[serde(default)] -> false
    assert!(!p.all);
}

#[test]
fn bulk_delete_tickets_params_with_author_filter() {
    let raw = r#"{"author_id":"u123"}"#;
    let p: BulkDeleteTicketsParams = serde_json::from_str(raw).unwrap();
    assert_eq!(p.author_id.as_deref(), Some("u123"));
    assert!(p.from.is_none());
    assert!(!p.all);
}

#[test]
fn bulk_delete_tickets_params_date_range() {
    let raw = r#"{"from":"2026-01-01","to":"2026-06-30"}"#;
    let p: BulkDeleteTicketsParams = serde_json::from_str(raw).unwrap();
    assert_eq!(p.from.as_deref(), Some("2026-01-01"));
    assert_eq!(p.to.as_deref(), Some("2026-06-30"));
}

#[test]
fn bulk_delete_tickets_params_all_flag() {
    let p: BulkDeleteTicketsParams = serde_json::from_str(r#"{"all":true}"#).unwrap();
    assert!(p.all);
    assert!(p.author_id.is_none());
}

#[test]
fn bulk_delete_tickets_params_combined_filters() {
    let raw = r#"{"author_id":"u","from":"2026-01-01T00:00:00Z","to":"2026-12-31T23:59:59Z","all":false}"#;
    let p: BulkDeleteTicketsParams = serde_json::from_str(raw).unwrap();
    assert_eq!(p.author_id.as_deref(), Some("u"));
    assert!(p.from.is_some());
    assert!(p.to.is_some());
    assert!(!p.all);
}
