use super::*;

#[test]
fn create_export_job_dto_defaults_empty_filters() {
    let raw = r#"{"guild_id":"g","requested_by":"u","job_type":"infractions","format":"csv"}"#;
    let dto: CreateExportJobDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g".into());
    assert_eq!(dto.job_type, "infractions");
    assert_eq!(dto.format, "csv");
    // filters absent → serde(default) → Null
    assert!(dto.filters.is_null());
}

#[test]
fn create_export_job_dto_with_filters() {
    let raw = r#"{"guild_id":"g","requested_by":"u","job_type":"audit_logs","format":"json","filters":{"limit":1000}}"#;
    let dto: CreateExportJobDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.format, "json");
    assert_eq!(dto.filters["limit"], 1000);
}

#[test]
fn export_job_created_dto_serializes() {
    let dto = ExportJobCreatedDto {
        job_id: "abc-123".into(),
        status: "pending".into(),
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"job_id\":\"abc-123\""));
    assert!(json.contains("\"status\":\"pending\""));
}
