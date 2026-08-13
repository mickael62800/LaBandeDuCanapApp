use super::*;

#[test]
fn test_as_str_roundtrip_all_variants() {
    let variants = vec![
        FlagType::Spam,
        FlagType::Insult,
        FlagType::Link,
        FlagType::Phishing,
        FlagType::Nsfw,
        FlagType::Illicit,
        FlagType::Anger,
        FlagType::Rage,
        FlagType::Threat,
        FlagType::Harassment,
    ];
    for v in variants {
        let s = v.as_str();
        let back = FlagType::from_str_lossy(s);
        assert_eq!(v, back, "Roundtrip failed for {s}");
    }
}

#[test]
fn test_from_str_lossy_unknown_defaults_spam() {
    assert_eq!(FlagType::from_str_lossy("unknown"), FlagType::Spam);
    assert_eq!(FlagType::from_str_lossy(""), FlagType::Spam);
}

#[test]
fn test_serde_serialize_snake_case() {
    let json = serde_json::to_string(&FlagType::Nsfw).unwrap();
    assert_eq!(json, "\"nsfw\"");

    let json = serde_json::to_string(&FlagType::Harassment).unwrap();
    assert_eq!(json, "\"harassment\"");
}

#[test]
fn test_serde_deserialize_snake_case() {
    let flag: FlagType = serde_json::from_str("\"threat\"").unwrap();
    assert_eq!(flag, FlagType::Threat);
}
