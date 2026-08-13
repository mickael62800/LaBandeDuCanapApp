use super::*;

// ── validate_purge_days_allow_zero ──

#[test]
fn allow_zero_accepts_positive() {
    assert!(validate_purge_days_allow_zero(1).is_ok());
    assert!(validate_purge_days_allow_zero(30).is_ok());
    assert!(validate_purge_days_allow_zero(i32::MAX).is_ok());
}

#[test]
fn allow_zero_accepts_zero_as_purge_all() {
    // 0 signifie "tout supprimer" pour les infractions.
    assert!(validate_purge_days_allow_zero(0).is_ok());
}

#[test]
fn allow_zero_rejects_negative() {
    assert_eq!(
        validate_purge_days_allow_zero(-1).unwrap_err(),
        "days doit etre >= 0"
    );
    assert_eq!(
        validate_purge_days_allow_zero(i32::MIN).unwrap_err(),
        "days doit etre >= 0"
    );
}

// ── validate_purge_days_strictly_positive ──

#[test]
fn strictly_positive_accepts_one_and_above() {
    assert!(validate_purge_days_strictly_positive(1).is_ok());
    assert!(validate_purge_days_strictly_positive(90).is_ok());
    assert!(validate_purge_days_strictly_positive(i32::MAX).is_ok());
}

#[test]
fn strictly_positive_rejects_zero() {
    // 0 serait "tout supprimer" mais refuse pour audit/system logs.
    assert_eq!(
        validate_purge_days_strictly_positive(0).unwrap_err(),
        "days doit etre >= 1"
    );
}

#[test]
fn strictly_positive_rejects_negative() {
    assert_eq!(
        validate_purge_days_strictly_positive(-5).unwrap_err(),
        "days doit etre >= 1"
    );
    assert_eq!(
        validate_purge_days_strictly_positive(i32::MIN).unwrap_err(),
        "days doit etre >= 1"
    );
}
