use super::*;

#[test]
fn model_info_serializes() {
    let info = ModelInfo {
        name: "ONNX Vision v1.0".into(),
        model_type: "vision".into(),
        loaded: true,
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"name\":\"ONNX Vision v1.0\""));
    assert!(json.contains("\"model_type\":\"vision\""));
    assert!(json.contains("\"loaded\":true"));
}

#[test]
fn models_status_response_empty() {
    let r = ModelsStatusResponse { models: vec![] };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"models\":[]"));
}

#[test]
fn models_status_response_multiple_models() {
    let r = ModelsStatusResponse {
        models: vec![
            ModelInfo {
                name: "a".into(),
                model_type: "text".into(),
                loaded: false,
            },
            ModelInfo {
                name: "b".into(),
                model_type: "vision".into(),
                loaded: true,
            },
        ],
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"text\""));
    assert!(json.contains("\"vision\""));
}

#[test]
fn reload_request_deserializes() {
    let req: ReloadRequest = serde_json::from_str(r#"{"model_type":"text"}"#).unwrap();
    assert_eq!(req.model_type, "text");
}

#[test]
fn reload_response_success_serializes() {
    let r = ReloadResponse {
        success: true,
        message: "Modele charge avec succes".into(),
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"success\":true"));
    assert!(json.contains("charge"));
}

#[test]
fn reload_response_error_serializes() {
    let r = ReloadResponse {
        success: false,
        message: "Modele introuvable".into(),
    };
    let json = serde_json::to_string(&r).unwrap();
    assert!(json.contains("\"success\":false"));
    assert!(json.contains("introuvable"));
}
