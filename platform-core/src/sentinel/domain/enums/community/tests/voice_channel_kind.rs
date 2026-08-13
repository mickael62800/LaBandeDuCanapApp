use super::*;

#[test]
fn as_str_public_and_private() {
    assert_eq!(VoiceChannelKind::Public.as_str(), "public");
    assert_eq!(VoiceChannelKind::Private.as_str(), "private");
}

#[test]
fn from_str_lossy_private() {
    assert_eq!(
        VoiceChannelKind::from_str_lossy("private"),
        VoiceChannelKind::Private
    );
}

#[test]
fn from_str_lossy_defaults_to_public() {
    assert_eq!(
        VoiceChannelKind::from_str_lossy("public"),
        VoiceChannelKind::Public
    );
    assert_eq!(
        VoiceChannelKind::from_str_lossy(""),
        VoiceChannelKind::Public
    );
    assert_eq!(
        VoiceChannelKind::from_str_lossy("unknown"),
        VoiceChannelKind::Public
    );
    assert_eq!(
        VoiceChannelKind::from_str_lossy("PRIVATE"),
        VoiceChannelKind::Public
    ); // case-sensitive
}

#[test]
fn default_is_public() {
    assert_eq!(VoiceChannelKind::default(), VoiceChannelKind::Public);
}

#[test]
fn roundtrip_via_as_str_and_from_str_lossy() {
    for k in [VoiceChannelKind::Public, VoiceChannelKind::Private] {
        assert_eq!(VoiceChannelKind::from_str_lossy(k.as_str()), k);
    }
}

#[test]
fn serde_snake_case() {
    let json = serde_json::to_string(&VoiceChannelKind::Private).unwrap();
    assert_eq!(json, "\"private\"");
    let back: VoiceChannelKind = serde_json::from_str(&json).unwrap();
    assert_eq!(back, VoiceChannelKind::Private);
}
