use super::*;

#[test]
fn roundtrip_all_variants() {
    for s in TicketStatus::VALID_VALUES {
        let status = TicketStatus::from_str(s).unwrap();
        assert_eq!(status.as_str(), *s);
    }
}

#[test]
fn from_str_invalid() {
    assert!(TicketStatus::from_str("invalid").is_none());
    assert!(TicketStatus::from_str("").is_none());
    assert!(TicketStatus::from_str("OPEN").is_none());
}

#[test]
fn serde_roundtrip() {
    let json = serde_json::to_string(&TicketStatus::Open).unwrap();
    assert_eq!(json, "\"open\"");
    let back: TicketStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(back, TicketStatus::Open);
}

#[test]
fn display_trait() {
    assert_eq!(format!("{}", TicketStatus::Closed), "closed");
}

#[test]
fn valid_values_count() {
    assert_eq!(TicketStatus::VALID_VALUES.len(), 3);
}

#[test]
fn can_transition_closing_always_allowed() {
    use TicketStatus::*;
    assert!(TicketStatus::can_transition(Open, Closed));
    assert!(TicketStatus::can_transition(Pending, Closed));
    assert!(TicketStatus::can_transition(Closed, Closed));
}

#[test]
fn can_transition_closed_cannot_reopen_via_pending() {
    use TicketStatus::*;
    // Une reponse (open/pending) ne doit pas reouvrir un ticket ferme.
    assert!(!TicketStatus::can_transition(Closed, Pending));
}

#[test]
fn can_transition_closed_reopen_to_open_allowed() {
    use TicketStatus::*;
    // Reouverture explicite autorisee.
    assert!(TicketStatus::can_transition(Closed, Open));
}

#[test]
fn can_transition_open_pending_free() {
    use TicketStatus::*;
    assert!(TicketStatus::can_transition(Open, Pending));
    assert!(TicketStatus::can_transition(Pending, Open));
    assert!(TicketStatus::can_transition(Open, Open));
    assert!(TicketStatus::can_transition(Pending, Pending));
}
