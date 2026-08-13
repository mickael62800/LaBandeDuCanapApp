use super::*;

#[test]
fn parse_permissions_valid_i64() {
    assert_eq!(parse_discord_permissions_bitfield("0"), 0);
    assert_eq!(parse_discord_permissions_bitfield("8"), 8);
    assert_eq!(parse_discord_permissions_bitfield("2147483648"), 2147483648);
}

#[test]
fn parse_permissions_fallback_on_invalid() {
    assert_eq!(parse_discord_permissions_bitfield(""), 0);
    assert_eq!(parse_discord_permissions_bitfield("abc"), 0);
    assert_eq!(parse_discord_permissions_bitfield("1.5"), 0);
}

#[test]
fn parse_permissions_accepts_large_bigint() {
    // Les permissions Discord peuvent depasser Number.MAX_SAFE_INTEGER (2^53).
    assert_eq!(
        parse_discord_permissions_bitfield("9007199254740993"),
        9007199254740993
    );
}

#[test]
fn parse_permissions_rejects_overflow_falls_back_to_zero() {
    // > i64::MAX → fallback 0.
    assert_eq!(
        parse_discord_permissions_bitfield("999999999999999999999"),
        0
    );
}

#[test]
fn parse_permissions_accepts_negative_i64() {
    // Techniquement les permissions Discord sont unsigned mais on stocke en
    // BIGINT signe → le parse accepte les negatifs.
    assert_eq!(parse_discord_permissions_bitfield("-1"), -1);
}
