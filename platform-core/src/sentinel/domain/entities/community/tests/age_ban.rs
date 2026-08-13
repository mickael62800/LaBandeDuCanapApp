//! Tests du statut de ban age (roundtrip + defaut fail-safe).

use super::*;

#[test]
fn as_str_values() {
    assert_eq!(AgeBanStatus::Pending.as_str(), "pending");
    assert_eq!(AgeBanStatus::Lifted.as_str(), "lifted");
}

#[test]
fn from_str_roundtrip() {
    assert_eq!(AgeBanStatus::from_str("pending"), AgeBanStatus::Pending);
    assert_eq!(AgeBanStatus::from_str("lifted"), AgeBanStatus::Lifted);
}

#[test]
fn from_str_unknown_defaults_to_pending() {
    // Fail-safe : tout statut inconnu reste un ban actif (pending), jamais leve.
    assert_eq!(AgeBanStatus::from_str(""), AgeBanStatus::Pending);
    assert_eq!(AgeBanStatus::from_str("garbage"), AgeBanStatus::Pending);
    assert_eq!(AgeBanStatus::from_str("Lifted"), AgeBanStatus::Pending);
}
