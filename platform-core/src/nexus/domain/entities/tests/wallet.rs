use crate::nexus::domain::entities::wallet::clamp_limit;
use crate::nexus::domain::entities::wallet::resolve_starting_coins;
use crate::nexus::domain::entities::wallet::validate_source;
use crate::nexus::domain::entities::wallet::validate_transfer;
use crate::nexus::domain::entities::wallet::Wallet;
use crate::nexus::domain::entities::wallet::DEFAULT_STARTING_COINS;
use crate::nexus::domain::entities::wallet::MAX_WALLET_AMOUNT;

#[test]
fn new_wallet_starts_at_zero() {
    let w = Wallet::new("g1", "u1");
    assert_eq!(w.coins, 0);
    assert_eq!(w.total_earned, 0);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn credit_increases_coins_and_total_earned() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(500).unwrap();
    w.credit(200).unwrap();
    assert_eq!(w.coins, 700);
    assert_eq!(w.total_earned, 700);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn credit_rejects_zero_and_negative() {
    let mut w = Wallet::new("g1", "u1");
    assert!(w.credit(0).is_err());
    assert!(w.credit(-10).is_err());
    assert_eq!(w.coins, 0);
}

#[test]
fn debit_removes_coins_and_tracks_total_spent() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(1000).unwrap();
    let actual = w.debit_clamped(300).unwrap();
    assert_eq!(actual, 300);
    assert_eq!(w.coins, 700);
    assert_eq!(w.total_spent, 300);
}

#[test]
fn debit_is_clamped_to_balance_never_negative() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(100).unwrap();
    let actual = w.debit_clamped(2000).unwrap();
    assert_eq!(actual, 100);
    assert_eq!(w.coins, 0);
    assert_eq!(w.total_spent, 100);
}

#[test]
fn debit_on_empty_wallet_debits_nothing() {
    let mut w = Wallet::new("g1", "u1");
    let actual = w.debit_clamped(500).unwrap();
    assert_eq!(actual, 0);
    assert_eq!(w.coins, 0);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn debit_rejects_zero_and_negative() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(100).unwrap();
    assert!(w.debit_clamped(0).is_err());
    assert!(w.debit_clamped(-5).is_err());
    assert_eq!(w.coins, 100);
}

#[test]
fn credit_saturates_instead_of_overflowing() {
    let mut w = Wallet::new("g1", "u1");
    w.coins = i64::MAX - 1;
    w.credit(1000).unwrap();
    assert_eq!(w.coins, i64::MAX);
}

// ── debit_exact (politique TRANSFERTS : refus, pas de clamp) ──

#[test]
fn debit_exact_removes_coins_when_sufficient() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(500).unwrap();
    w.debit_exact(200).unwrap();
    assert_eq!(w.coins, 300);
    assert_eq!(w.total_spent, 200);
}

#[test]
fn debit_exact_refuses_insufficient_balance_without_clamping() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(100).unwrap();
    assert!(w.debit_exact(101).is_err());
    // Refus explicite : le wallet est intact, pas de debit partiel.
    assert_eq!(w.coins, 100);
    assert_eq!(w.total_spent, 0);
}

#[test]
fn debit_exact_rejects_zero_and_negative() {
    let mut w = Wallet::new("g1", "u1");
    w.credit(100).unwrap();
    assert!(w.debit_exact(0).is_err());
    assert!(w.debit_exact(-5).is_err());
    assert_eq!(w.coins, 100);
}

// ── resolve_starting_coins ──

#[test]
fn starting_coins_default_is_historical_100() {
    assert_eq!(DEFAULT_STARTING_COINS, 100);
    assert_eq!(resolve_starting_coins(None), 100);
}

#[test]
fn starting_coins_uses_guild_override() {
    assert_eq!(resolve_starting_coins(Some(2500)), 2500);
    assert_eq!(resolve_starting_coins(Some(0)), 0);
}

#[test]
fn starting_coins_never_negative() {
    assert_eq!(resolve_starting_coins(Some(-50)), 0);
}

// ── validate_transfer (regles /donner) ──

#[test]
fn transfer_rejects_self_transfer() {
    assert!(validate_transfer("u1", "u1", 10, 1000).is_err());
}

#[test]
fn transfer_rejects_zero_and_negative_amount() {
    assert!(validate_transfer("u1", "u2", 0, 1000).is_err());
    assert!(validate_transfer("u1", "u2", -10, 1000).is_err());
}

#[test]
fn transfer_rejects_amount_above_historical_cap() {
    assert!(validate_transfer("u1", "u2", MAX_WALLET_AMOUNT + 1, i64::MAX).is_err());
    assert!(validate_transfer("u1", "u2", MAX_WALLET_AMOUNT, i64::MAX).is_ok());
}

#[test]
fn transfer_refuses_insufficient_balance_explicitly() {
    let err = validate_transfer("u1", "u2", 101, 100).unwrap_err();
    assert!(matches!(
        err,
        crate::nexus::domain::errors::DomainError::Validation(_)
    ));
}

#[test]
fn transfer_accepts_valid_command() {
    assert!(validate_transfer("u1", "u2", 100, 100).is_ok());
}

// ── validate_source / clamp_limit ──

#[test]
fn source_must_be_non_empty_and_short() {
    assert!(validate_source("").is_err());
    assert!(validate_source("   ").is_err());
    assert!(validate_source(&"x".repeat(41)).is_err());
    assert!(validate_source("wheel_payout").is_ok());
    assert!(validate_source("admin_grant").is_ok());
}

#[test]
fn clamp_limit_applies_default_and_bounds() {
    assert_eq!(clamp_limit(None, 10, 50), 10);
    assert_eq!(clamp_limit(Some(25), 10, 50), 25);
    assert_eq!(clamp_limit(Some(0), 10, 50), 1);
    assert_eq!(clamp_limit(Some(-3), 10, 50), 1);
    assert_eq!(clamp_limit(Some(999), 10, 50), 50);
}
