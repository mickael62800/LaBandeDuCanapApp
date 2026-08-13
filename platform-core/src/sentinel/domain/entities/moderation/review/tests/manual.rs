use super::*;

// ── validate_evidence_url ──

#[test]
fn evidence_url_accepts_valid() {
    assert!(validate_evidence_url("https://example.com/proof.png").is_ok());
}

#[test]
fn evidence_url_rejects_empty() {
    assert_eq!(
        validate_evidence_url("").unwrap_err(),
        "url vide ou trop longue (max 2000)"
    );
}

#[test]
fn evidence_url_rejects_whitespace_only() {
    assert!(validate_evidence_url("   ").is_err());
    assert!(validate_evidence_url("\t\n").is_err());
}

#[test]
fn evidence_url_accepts_exactly_2000_chars() {
    let url = "https://x.com/".to_string() + &"a".repeat(2000 - 14);
    assert_eq!(url.len(), 2000);
    assert!(validate_evidence_url(&url).is_ok());
}

#[test]
fn evidence_url_rejects_over_2000_chars() {
    let too_long = "a".repeat(2001);
    assert!(validate_evidence_url(&too_long).is_err());
}

// ── truncate_review_text ──

#[test]
fn truncate_preserves_short_text() {
    assert_eq!(truncate_review_text("Hello"), "Hello");
    assert_eq!(truncate_review_text(""), "");
}

#[test]
fn truncate_cuts_at_500_chars() {
    let long = "a".repeat(600);
    let out = truncate_review_text(&long);
    assert_eq!(out.chars().count(), 500);
}

#[test]
fn truncate_counts_unicode_graphemes() {
    let input = "é".repeat(600);
    let out = truncate_review_text(&input);
    assert_eq!(out.chars().count(), 500);
    // Mais cote octets, "é" = 2 octets → 1000 octets
    assert_eq!(out.len(), 1000);
}

#[test]
fn truncate_preserves_exactly_500_chars() {
    let exact = "x".repeat(500);
    assert_eq!(truncate_review_text(&exact).len(), 500);
}

// ── is_valid_review_status ──

#[test]
fn review_status_accepts_three_values() {
    assert!(is_valid_review_status("approved"));
    assert!(is_valid_review_status("rejected"));
    assert!(is_valid_review_status("changed"));
}

#[test]
fn review_status_rejects_others() {
    assert!(!is_valid_review_status("pending"));
    assert!(!is_valid_review_status("resolved"));
    assert!(!is_valid_review_status(""));
    assert!(!is_valid_review_status("APPROVED")); // case-sensitive
}

#[test]
fn default_mute_duration_is_one_hour() {
    assert_eq!(DEFAULT_MUTE_DURATION_SECS, 3600);
}

#[test]
fn resolve_mute_duration_none_uses_default() {
    assert_eq!(resolve_mute_duration(None), DEFAULT_MUTE_DURATION_SECS);
    assert_eq!(resolve_mute_duration(None), 3600);
}

#[test]
fn resolve_mute_duration_preserves_some_value() {
    assert_eq!(resolve_mute_duration(Some(60)), 60);
    assert_eq!(resolve_mute_duration(Some(86_400)), 86_400);
}

#[test]
fn resolve_mute_duration_accepts_zero_as_explicit_no_op() {
    // Defensif : Some(0) est une valeur explicite, on ne la remplace pas.
    // Discord clampera cote API si invalide.
    assert_eq!(resolve_mute_duration(Some(0)), 0);
}

#[test]
fn valid_review_statuses_contains_three() {
    assert_eq!(VALID_REVIEW_STATUSES.len(), 3);
}
