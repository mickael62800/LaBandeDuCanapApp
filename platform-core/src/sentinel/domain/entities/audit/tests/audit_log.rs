use super::*;

#[test]
fn nickname_history_event_name_is_stable() {
    // Regle metier : l'identifiant est stable, consomme par desktop/exports.
    assert_eq!(
        AUDIT_EVENT_MEMBER_NICKNAME_HISTORY,
        "member_nickname_history"
    );
}

#[test]
fn security_prefix_matches_known_events() {
    assert_eq!(AUDIT_EVENT_SECURITY_PREFIX, "security_");
}

#[test]
fn is_security_event_accepts_prefixed() {
    assert!(is_security_audit_event("security_raid"));
    assert!(is_security_audit_event("security_alt_detected"));
    assert!(is_security_audit_event("security_")); // prefix seul (edge case)
}

#[test]
fn is_security_event_rejects_others() {
    assert!(!is_security_audit_event("member_nickname_history"));
    assert!(!is_security_audit_event("role.update"));
    assert!(!is_security_audit_event(""));
    assert!(!is_security_audit_event("SECURITY_RAID")); // case-sensitive
}
