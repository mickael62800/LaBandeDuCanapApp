use super::*;

#[test]
fn ai_job_types_whitelist_has_two() {
    assert_eq!(VALID_AI_JOB_TYPES.len(), 2);
    assert!(VALID_AI_JOB_TYPES.contains(&"analyze_text"));
    assert!(VALID_AI_JOB_TYPES.contains(&"analyze_image"));
}

#[test]
fn ai_job_type_accepts_known() {
    assert!(is_valid_ai_job_type("analyze_text"));
    assert!(is_valid_ai_job_type("analyze_image"));
}

#[test]
fn ai_job_type_rejects_unknown() {
    assert!(!is_valid_ai_job_type("analyze_video"));
    assert!(!is_valid_ai_job_type(""));
    assert!(!is_valid_ai_job_type("ANALYZE_TEXT")); // case-sensitive
}

#[test]
fn export_job_types_whitelist_has_three() {
    assert_eq!(VALID_EXPORT_JOB_TYPES.len(), 3);
    assert!(VALID_EXPORT_JOB_TYPES.contains(&"infractions"));
    assert!(VALID_EXPORT_JOB_TYPES.contains(&"audit_logs"));
    assert!(VALID_EXPORT_JOB_TYPES.contains(&"moderation_actions"));
}

#[test]
fn export_job_type_accepts_known() {
    assert!(is_valid_export_job_type("infractions"));
    assert!(is_valid_export_job_type("audit_logs"));
    assert!(is_valid_export_job_type("moderation_actions"));
}

#[test]
fn export_job_type_rejects_unknown() {
    assert!(!is_valid_export_job_type("tickets"));
    assert!(!is_valid_export_job_type(""));
}

#[test]
fn export_formats_are_csv_and_json() {
    assert_eq!(VALID_EXPORT_FORMATS.len(), 2);
    assert!(is_valid_export_format("csv"));
    assert!(is_valid_export_format("json"));
}

#[test]
fn export_format_rejects_unknown() {
    assert!(!is_valid_export_format("xml"));
    assert!(!is_valid_export_format("yaml"));
    assert!(!is_valid_export_format(""));
    assert!(!is_valid_export_format("CSV")); // case-sensitive
}
