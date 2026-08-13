use super::*;

#[test]
fn pagination_query_empty_all_none() {
    let q: PaginationQuery = serde_json::from_str(r#"{}"#).unwrap();
    assert!(q.limit.is_none());
    assert!(q.offset.is_none());
}

#[test]
fn pagination_query_with_limit() {
    let q: PaginationQuery = serde_json::from_str(r#"{"limit":50}"#).unwrap();
    assert_eq!(q.limit, Some(50));
    assert!(q.offset.is_none());
}

#[test]
fn pagination_query_with_offset() {
    let q: PaginationQuery = serde_json::from_str(r#"{"offset":100}"#).unwrap();
    assert!(q.limit.is_none());
    assert_eq!(q.offset, Some(100));
}

#[test]
fn pagination_query_full() {
    let q: PaginationQuery = serde_json::from_str(r#"{"limit":25,"offset":50}"#).unwrap();
    assert_eq!(q.limit, Some(25));
    assert_eq!(q.offset, Some(50));
}
