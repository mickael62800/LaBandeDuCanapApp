//! Tests de la validation de `config_key`.

use super::*;

#[test]
fn accepts_valid_keys() {
    assert!(validate_config_key("MAX").is_ok());
    assert!(validate_config_key("MAX_PLAYERS").is_ok());
    assert!(validate_config_key("LEVEL_2_CAP").is_ok());
    assert!(validate_config_key("A").is_ok());
}

#[test]
fn accepts_mixed_case_after_first_char() {
    // Noms imposes par l'image 7 Days to Die, lus tels quels par le serveur.
    assert!(validate_config_key("SERVERCONFIG_BuildCreate").is_ok());
    assert!(validate_config_key("SERVERCONFIG_ZombieMove").is_ok());
    assert!(validate_config_key("SERVERCONFIG_XPMultiplier").is_ok());
}

#[test]
fn rejects_empty() {
    assert!(validate_config_key("").is_err());
}

#[test]
fn rejects_too_long() {
    // 64 = borne haute acceptee, 65 = refuse.
    let ok = "A".repeat(64);
    let too_long = "A".repeat(65);
    assert!(validate_config_key(&ok).is_ok());
    assert!(validate_config_key(&too_long).is_err());
}

#[test]
fn rejects_lowercase_first_char() {
    assert!(validate_config_key("mAX").is_err());
    assert!(validate_config_key("_MAX").is_err());
    assert!(validate_config_key("2MAX").is_err());
}

#[test]
fn rejects_invalid_body_chars() {
    assert!(validate_config_key("MAX-PLAYERS").is_err());
    assert!(validate_config_key("MAX PLAYERS").is_err());
    assert!(validate_config_key("MAX.PLAYERS").is_err());
    assert!(validate_config_key("MAX/PLAYERS").is_err());
}
