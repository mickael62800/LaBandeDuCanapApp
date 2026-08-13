use super::*;

#[test]
fn csv_escape_simple() {
    assert_eq!(csv_escape("hello"), "hello");
}

#[test]
fn csv_escape_comma() {
    assert_eq!(csv_escape("a,b"), "\"a,b\"");
}

#[test]
fn csv_escape_quote() {
    assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
}

#[test]
fn csv_escape_newline() {
    assert_eq!(csv_escape("a\nb"), "\"a\nb\"");
}

#[test]
fn to_csv_basic() {
    let rows = vec![
        ("1".to_string(), "hello".to_string()),
        ("2".to_string(), "a,b".to_string()),
    ];
    let csv = to_csv(&rows, &["id", "val"], |r| vec![r.0.clone(), r.1.clone()]);
    assert_eq!(csv, "id,val\n1,hello\n2,\"a,b\"\n");
}

#[test]
fn to_csv_empty() {
    let rows: Vec<(String,)> = vec![];
    let csv = to_csv(&rows, &["id"], |r| vec![r.0.clone()]);
    assert_eq!(csv, "id\n");
}

#[test]
fn serialize_rows_json() {
    let rows = vec![serde_json::json!({"a": 1}), serde_json::json!({"a": 2})];
    let result = serialize_rows(&rows, "json", |_| vec![], &[]).unwrap();
    assert_eq!(result.row_count, 2);
    assert!(result.data.contains("\"a\":1") || result.data.contains("\"a\": 1"));
}

#[test]
fn serialize_rows_unknown_format() {
    let rows: Vec<String> = vec![];
    let result = serialize_rows(&rows, "xml", |_| vec![], &[]);
    assert!(result.is_err());
}

#[test]
fn csv_escape_tab_not_quoted() {
    // Les tabs n'exigent pas de quoting (seulement virgule, guillemet, newline).
    assert_eq!(csv_escape("a\tb"), "a\tb");
}

#[test]
fn csv_escape_empty_field() {
    assert_eq!(csv_escape(""), "");
}

#[test]
fn csv_escape_multiple_quotes() {
    assert_eq!(csv_escape("\"a\"b\""), "\"\"\"a\"\"b\"\"\"");
}

#[test]
fn to_csv_three_rows_with_mixed_escaping() {
    let rows = vec![
        ("1".to_string(), "plain".to_string()),
        ("2".to_string(), "with,comma".to_string()),
        ("3".to_string(), "has \"quotes\"".to_string()),
    ];
    let csv = to_csv(&rows, &["id", "val"], |r| vec![r.0.clone(), r.1.clone()]);
    assert!(csv.starts_with("id,val\n"));
    assert!(csv.contains("1,plain\n"));
    assert!(csv.contains("\"with,comma\""));
    assert!(csv.contains("\"has \"\"quotes\"\"\""));
}

#[test]
fn serialize_rows_csv_format_produces_header_and_rows() {
    let rows = vec![
        ("a".to_string(), "1".to_string()),
        ("b".to_string(), "2".to_string()),
    ];
    let result = serialize_rows(
        &rows,
        "csv",
        |r| vec![r.0.clone(), r.1.clone()],
        &["name", "val"],
    )
    .unwrap();
    assert_eq!(result.row_count, 2);
    assert!(result.data.starts_with("name,val\n"));
    assert!(result.data.contains("a,1"));
    assert!(result.data.contains("b,2"));
}

// ── execute() dispatch ──

struct StubExportRepo;
#[async_trait::async_trait]
impl ExportRepository for StubExportRepo {
    async fn fetch_infractions(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<InfractionExport>, DomainError> {
        unimplemented!()
    }
    async fn fetch_audit_logs(&self, _: &str, _: i64) -> Result<Vec<AuditLogExport>, DomainError> {
        unimplemented!()
    }
    async fn fetch_moderation_actions(
        &self,
        _: &str,
        _: i64,
    ) -> Result<Vec<ModerationActionExport>, DomainError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn execute_rejects_unknown_job_type() {
    // Repo inutile pour cette branche : ValidationError se leve avant la requete
    // (job_type inconnu court-circuite avant tout acces au repository).
    let svc = ExportService::new(std::sync::Arc::new(StubExportRepo));
    let err = svc
        .execute("g", "unknown_type", "csv", 100)
        .await
        .unwrap_err();
    match err {
        DomainError::ValidationError(m) => assert!(m.contains("job_type inconnu")),
        other => panic!("Expected ValidationError, got {:?}", other),
    }
}
