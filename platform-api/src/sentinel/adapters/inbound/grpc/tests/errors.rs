use super::*;

use uuid::Uuid;

#[test]
fn rule_not_found_maps_to_not_found() {
    let s = domain_to_status(DomainError::NotFound(format!("Regle {}", Uuid::nil())));
    assert_eq!(s.code(), Code::NotFound);
}

#[test]
fn infraction_not_found_maps_to_not_found() {
    let s = domain_to_status(DomainError::NotFound(format!("Infraction {}", Uuid::nil())));
    assert_eq!(s.code(), Code::NotFound);
}

#[test]
fn ticket_not_found_maps_to_not_found() {
    let s = domain_to_status(DomainError::NotFound("Ticket t1".to_string()));
    assert_eq!(s.code(), Code::NotFound);
}

#[test]
fn generic_not_found_maps_to_not_found() {
    let s = domain_to_status(DomainError::NotFound("absent".into()));
    assert_eq!(s.code(), Code::NotFound);
    assert!(s.message().contains("absent"));
}

#[test]
fn invalid_rule_maps_to_invalid_argument() {
    let s = domain_to_status(DomainError::ValidationError("bad regex".into()));
    assert_eq!(s.code(), Code::InvalidArgument);
}

#[test]
fn validation_error_maps_to_invalid_argument() {
    let s = domain_to_status(DomainError::ValidationError("champ invalide".into()));
    assert_eq!(s.code(), Code::InvalidArgument);
    assert!(s.message().contains("invalide"));
}

#[test]
fn conflict_maps_to_already_exists() {
    let s = domain_to_status(DomainError::Conflict("deja la".into()));
    assert_eq!(s.code(), Code::AlreadyExists);
}

#[test]
fn forbidden_maps_to_permission_denied() {
    let s = domain_to_status(DomainError::Forbidden("interdit".into()));
    assert_eq!(s.code(), Code::PermissionDenied);
}

#[test]
fn rate_limited_maps_to_resource_exhausted() {
    let s = domain_to_status(DomainError::RateLimited("trop vite".into()));
    assert_eq!(s.code(), Code::ResourceExhausted);
}

#[test]
fn timeout_maps_to_deadline_exceeded() {
    let s = domain_to_status(DomainError::Timeout("trop long".into()));
    assert_eq!(s.code(), Code::DeadlineExceeded);
}

#[test]
fn internal_maps_to_internal() {
    let s = domain_to_status(DomainError::Internal("oops".into()));
    assert_eq!(s.code(), Code::Internal);
}

// ── sqlx_to_status ──

#[test]
fn sqlx_to_status_wraps_with_context() {
    let mapper = sqlx_to_status("INSERT users");
    let s = mapper(sqlx::Error::RowNotFound);
    assert_eq!(s.code(), Code::Internal);
    assert!(s.message().starts_with("INSERT users: "));
}

#[test]
fn sqlx_to_status_different_contexts_are_independent() {
    let m1 = sqlx_to_status("INSERT A");
    let m2 = sqlx_to_status("SELECT B");
    let s1 = m1(sqlx::Error::PoolClosed);
    let s2 = m2(sqlx::Error::PoolClosed);
    assert!(s1.message().starts_with("INSERT A:"));
    assert!(s2.message().starts_with("SELECT B:"));
}

#[test]
fn sqlx_to_status_closure_reusable() {
    let mapper = sqlx_to_status("ctx");
    // Le closure implemente Fn → reutilisable plusieurs fois.
    let _s1 = mapper(sqlx::Error::RowNotFound);
    let _s2 = mapper(sqlx::Error::PoolClosed);
}
