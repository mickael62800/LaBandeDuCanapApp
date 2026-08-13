use super::*;

#[test]
fn config_default_values() {
    let c = VoiceChannelConfig::default();
    assert_eq!(c.creation_cooldown_secs, 5);
    assert_eq!(c.flood_max_messages, 5);
    assert_eq!(c.flood_time_window_secs, 5);
    assert_eq!(c.empty_cleanup_delay_secs, 2);
    assert_eq!(c.flood_mute_duration_secs, 30);
    assert_eq!(c.vote_kick_timeout_secs, 60);
}

#[test]
fn from_kv_pairs_empty_returns_default() {
    let c = VoiceChannelConfig::from_kv_pairs(&[]);
    let d = VoiceChannelConfig::default();
    assert_eq!(c.creation_cooldown_secs, d.creation_cooldown_secs);
    assert_eq!(c.vote_kick_timeout_secs, d.vote_kick_timeout_secs);
}

#[test]
fn from_kv_pairs_parses_known_keys() {
    let pairs = vec![
        ("voice_creation_cooldown_secs".into(), "10".into()),
        ("voice_flood_max_messages".into(), "8".into()),
        ("voice_flood_time_window_secs".into(), "3".into()),
        ("voice_empty_cleanup_delay_secs".into(), "1".into()),
        ("voice_flood_mute_duration_secs".into(), "60".into()),
        ("voice_vote_kick_timeout_secs".into(), "120".into()),
    ];
    let c = VoiceChannelConfig::from_kv_pairs(&pairs);
    assert_eq!(c.creation_cooldown_secs, 10);
    assert_eq!(c.flood_max_messages, 8);
    assert_eq!(c.flood_time_window_secs, 3);
    assert_eq!(c.empty_cleanup_delay_secs, 1);
    assert_eq!(c.flood_mute_duration_secs, 60);
    assert_eq!(c.vote_kick_timeout_secs, 120);
}

#[test]
fn from_kv_pairs_ignores_unknown_keys() {
    let pairs = vec![
        ("unknown_key".into(), "99".into()),
        ("voice_creation_cooldown_secs".into(), "42".into()),
    ];
    let c = VoiceChannelConfig::from_kv_pairs(&pairs);
    assert_eq!(c.creation_cooldown_secs, 42);
    // Les autres valeurs restent par defaut.
    assert_eq!(c.flood_max_messages, 5);
}

#[test]
fn from_kv_pairs_ignores_invalid_values() {
    let pairs = vec![
        ("voice_creation_cooldown_secs".into(), "not_a_number".into()),
        ("voice_flood_max_messages".into(), "".into()),
    ];
    let c = VoiceChannelConfig::from_kv_pairs(&pairs);
    let d = VoiceChannelConfig::default();
    assert_eq!(c.creation_cooldown_secs, d.creation_cooldown_secs);
    assert_eq!(c.flood_max_messages, d.flood_max_messages);
}

#[test]
fn from_kv_pairs_last_wins_on_duplicate_key() {
    let pairs = vec![
        ("voice_creation_cooldown_secs".into(), "10".into()),
        ("voice_creation_cooldown_secs".into(), "20".into()),
    ];
    let c = VoiceChannelConfig::from_kv_pairs(&pairs);
    assert_eq!(c.creation_cooldown_secs, 20);
}

#[test]
fn from_kv_pairs_parse_errors_keep_defaults_for_all_keys() {
    // Couvre la branche "parse fail" (if let Err) pour chaque clef : si la
    // valeur n'est pas un nombre valide, on garde le default au lieu de set.
    let pairs = vec![
        ("voice_creation_cooldown_secs".into(), "abc".into()),
        ("voice_flood_max_messages".into(), "xyz".into()),
        ("voice_flood_time_window_secs".into(), "-".into()),
        ("voice_empty_cleanup_delay_secs".into(), "not_int".into()),
        ("voice_flood_mute_duration_secs".into(), "3.14".into()),
        ("voice_vote_kick_timeout_secs".into(), "infinity".into()),
    ];
    let c = VoiceChannelConfig::from_kv_pairs(&pairs);
    let d = VoiceChannelConfig::default();
    assert_eq!(c.creation_cooldown_secs, d.creation_cooldown_secs);
    assert_eq!(c.flood_max_messages, d.flood_max_messages);
    assert_eq!(c.flood_time_window_secs, d.flood_time_window_secs);
    assert_eq!(c.empty_cleanup_delay_secs, d.empty_cleanup_delay_secs);
    assert_eq!(c.flood_mute_duration_secs, d.flood_mute_duration_secs);
    assert_eq!(c.vote_kick_timeout_secs, d.vote_kick_timeout_secs);
}

#[test]
fn from_kv_pairs_partial_valid_partial_invalid() {
    // Melange : certaines clefs parsent, d'autres echouent → les valides
    // sont appliquees, les invalides restent au default.
    let pairs = vec![
        ("voice_creation_cooldown_secs".into(), "42".into()),
        ("voice_flood_max_messages".into(), "BAD".into()), // reste default
        ("voice_vote_kick_timeout_secs".into(), "99".into()),
    ];
    let c = VoiceChannelConfig::from_kv_pairs(&pairs);
    let d = VoiceChannelConfig::default();
    assert_eq!(c.creation_cooldown_secs, 42);
    assert_eq!(c.flood_max_messages, d.flood_max_messages);
    assert_eq!(c.vote_kick_timeout_secs, 99);
}
