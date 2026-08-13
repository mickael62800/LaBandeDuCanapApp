use super::*;

#[test]
fn create_ai_job_dto_analyze_text() {
    let raw = r#"{"guild_id":"g","job_type":"analyze_text","input_payload":{"text":"hello"}}"#;
    let dto: CreateAiJobDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.guild_id, "g");
    assert_eq!(dto.job_type, "analyze_text");
    assert_eq!(dto.input_payload["text"], "hello");
}

#[test]
fn create_ai_job_dto_analyze_image() {
    let raw = r#"{"guild_id":"g","job_type":"analyze_image","input_payload":{"url":"x/y.png"}}"#;
    let dto: CreateAiJobDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.job_type, "analyze_image");
    assert_eq!(dto.input_payload["url"], "x/y.png");
}

#[test]
fn create_ai_job_dto_arbitrary_payload() {
    let raw = r#"{"guild_id":"g","job_type":"t","input_payload":[1,2,3]}"#;
    let dto: CreateAiJobDto = serde_json::from_str(raw).unwrap();
    assert!(dto.input_payload.is_array());
}

#[test]
fn ai_job_created_dto_serializes() {
    let dto = AiJobCreatedDto {
        job_id: "uuid-123".into(),
        status: "pending".into(),
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"job_id\":\"uuid-123\""));
    assert!(json.contains("\"status\":\"pending\""));
}
