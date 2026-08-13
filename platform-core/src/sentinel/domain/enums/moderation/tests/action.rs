use super::*;

#[test]
fn test_as_str_roundtrip() {
    let actions = vec![
        Action::None,
        Action::Warn,
        Action::Delete,
        Action::Mute,
        Action::Ban,
    ];
    for a in actions {
        let s = a.as_str();
        let back = Action::from_str_lossy(s);
        assert_eq!(a, back);
    }
}

#[test]
fn test_from_str_lossy_unknown_defaults_none() {
    assert_eq!(Action::from_str_lossy("unknown"), Action::None);
    assert_eq!(Action::from_str_lossy(""), Action::None);
}

#[test]
fn test_action_ordering() {
    assert!(Action::Ban > Action::Mute);
    assert!(Action::Mute > Action::Delete);
    assert!(Action::Delete > Action::Warn);
    assert!(Action::Warn > Action::None);
}

#[test]
fn test_serde_roundtrip() {
    let json = serde_json::to_string(&Action::Ban).unwrap();
    assert_eq!(json, "\"ban\"");
    let back: Action = serde_json::from_str(&json).unwrap();
    assert_eq!(back, Action::Ban);
}
