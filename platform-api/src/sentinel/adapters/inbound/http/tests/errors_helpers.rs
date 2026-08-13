use super::*;
use platform_core::sentinel::domain::errors::DomainError;

#[test]
fn sqlx_internal_wraps_with_context() {
    let mapper = sqlx_internal("fetch voice");
    let err: DomainError = mapper(sqlx::Error::RowNotFound);
    match err {
        DomainError::Internal(msg) => {
            assert!(msg.starts_with("fetch voice:"));
        }
        other => panic!("expected Internal, got {other:?}"),
    }
}

#[test]
fn internal_with_wraps_display_error() {
    let mapper = internal_with::<String>("context");
    let err = mapper("boom".to_string());
    match err {
        DomainError::Internal(msg) => {
            assert_eq!(msg, "context: boom");
        }
        other => panic!("expected Internal, got {other:?}"),
    }
}

#[test]
fn validation_with_wraps_display_error() {
    let mapper = validation_with::<&str>("field");
    let err = mapper("too short");
    match err {
        DomainError::ValidationError(msg) => {
            assert_eq!(msg, "field: too short");
        }
        other => panic!("expected ValidationError, got {other:?}"),
    }
}

#[test]
fn internal_with_works_with_different_display_types() {
    // i32 implemente Display
    let mapper = internal_with::<i32>("code");
    let err = mapper(404);
    assert!(matches!(err, DomainError::Internal(ref m) if m == "code: 404"));
}
