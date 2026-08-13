use super::*;

#[test]
fn supported_types_are_vision_and_text() {
    assert_eq!(SUPPORTED_MODEL_TYPES, &["vision", "text"]);
}

#[test]
fn is_valid_model_type_accepts_whitelist() {
    assert!(is_valid_model_type("vision"));
    assert!(is_valid_model_type("text"));
    assert!(!is_valid_model_type("audio"));
    assert!(!is_valid_model_type(""));
    assert!(!is_valid_model_type("VISION"));
}

#[test]
fn path_basename_handles_unix_separator() {
    assert_eq!(path_basename("/opt/models/vision.onnx"), "vision.onnx");
}

#[test]
fn path_basename_handles_windows_separator() {
    assert_eq!(path_basename(r"C:\models\text.onnx"), "text.onnx");
}

#[test]
fn path_basename_handles_mixed_separators() {
    assert_eq!(path_basename(r"C:/models\sub/x.onnx"), "x.onnx");
}

#[test]
fn path_basename_no_separator_returns_input() {
    assert_eq!(path_basename("file.onnx"), "file.onnx");
}

#[test]
fn path_basename_empty_stays_empty() {
    assert_eq!(path_basename(""), "");
}

#[test]
fn format_model_display_name_empty_path() {
    assert_eq!(
        format_model_display_name("Vision", ""),
        "Vision ONNX (non configure)"
    );
}

#[test]
fn format_model_display_name_with_unix_path() {
    assert_eq!(
        format_model_display_name("Text", "/opt/models/bert.onnx"),
        "Text ONNX (bert.onnx)"
    );
}

#[test]
fn format_model_display_name_with_windows_path() {
    assert_eq!(
        format_model_display_name("Vision", r"C:\models\yolo.onnx"),
        "Vision ONNX (yolo.onnx)"
    );
}
