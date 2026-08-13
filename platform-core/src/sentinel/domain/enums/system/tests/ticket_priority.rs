use super::*;

#[test]
fn roundtrip_all_variants() {
    for s in TicketPriority::VALID_VALUES {
        let prio = TicketPriority::from_str(s).unwrap();
        assert_eq!(prio.as_str(), *s);
    }
}

#[test]
fn from_str_invalid() {
    assert!(TicketPriority::from_str("critical").is_none());
    assert!(TicketPriority::from_str("").is_none());
}

#[test]
fn ordering() {
    assert!(TicketPriority::Urgent > TicketPriority::High);
    assert!(TicketPriority::High > TicketPriority::Medium);
    assert!(TicketPriority::Medium > TicketPriority::Low);
}

#[test]
fn serde_roundtrip() {
    let json = serde_json::to_string(&TicketPriority::Urgent).unwrap();
    assert_eq!(json, "\"urgent\"");
    let back: TicketPriority = serde_json::from_str(&json).unwrap();
    assert_eq!(back, TicketPriority::Urgent);
}

#[test]
fn valid_values_count() {
    assert_eq!(TicketPriority::VALID_VALUES.len(), 4);
}

#[test]
fn display_trait_uses_as_str() {
    assert_eq!(format!("{}", TicketPriority::Low), "low");
    assert_eq!(format!("{}", TicketPriority::Medium), "medium");
    assert_eq!(format!("{}", TicketPriority::High), "high");
    assert_eq!(format!("{}", TicketPriority::Urgent), "urgent");
}

#[test]
fn as_str_covers_all_variants_directly() {
    assert_eq!(TicketPriority::Low.as_str(), "low");
    assert_eq!(TicketPriority::Medium.as_str(), "medium");
    assert_eq!(TicketPriority::High.as_str(), "high");
    assert_eq!(TicketPriority::Urgent.as_str(), "urgent");
}

#[test]
fn copy_and_clone() {
    let p = TicketPriority::High;
    let copy = p;
    let cloned = p;
    assert_eq!(p, copy);
    assert_eq!(p, cloned);
}

#[test]
fn equality_total_ordering() {
    use std::cmp::Ordering;
    assert_eq!(
        TicketPriority::Low.cmp(&TicketPriority::Low),
        Ordering::Equal
    );
    assert_eq!(
        TicketPriority::Low.cmp(&TicketPriority::Urgent),
        Ordering::Less
    );
    assert_eq!(
        TicketPriority::Urgent.cmp(&TicketPriority::Low),
        Ordering::Greater
    );
}

#[test]
fn valid_values_order_matches_enum() {
    // Les VALID_VALUES doivent suivre l'ordre croissant de l'enum.
    let expected = [
        TicketPriority::Low.as_str(),
        TicketPriority::Medium.as_str(),
        TicketPriority::High.as_str(),
        TicketPriority::Urgent.as_str(),
    ];
    assert_eq!(TicketPriority::VALID_VALUES, expected);
}
