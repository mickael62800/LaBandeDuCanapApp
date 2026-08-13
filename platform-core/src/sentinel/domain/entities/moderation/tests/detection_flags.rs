use super::*;

#[test]
fn test_no_active_flags() {
    let flags = DetectionFlags {
        spam: false,
        insult: false,
        profanity: false,
        link: false,
        phishing: false,
    };
    assert!(flags.active_flags().is_empty());
}

#[test]
fn test_all_active_flags() {
    let flags = DetectionFlags {
        spam: true,
        insult: true,
        profanity: false,
        link: true,
        phishing: true,
    };
    let active = flags.active_flags();
    assert_eq!(active.len(), 4);
}

#[test]
fn test_single_flag_spam() {
    let flags = DetectionFlags {
        spam: true,
        insult: false,
        profanity: false,
        link: false,
        phishing: false,
    };
    let active = flags.active_flags();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0], FlagType::Spam);
}

#[test]
fn test_phishing_default_false_serde() {
    let json = r#"{"spam": true, "insult": false, "link": false}"#;
    let flags: DetectionFlags = serde_json::from_str(json).unwrap();
    assert!(flags.spam);
    assert!(!flags.phishing); // default
}
