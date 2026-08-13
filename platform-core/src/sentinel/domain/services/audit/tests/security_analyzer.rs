use super::*;

#[test]
fn levenshtein_identical() {
    assert_eq!(levenshtein("hello", "hello"), 0);
}

#[test]
fn levenshtein_one_diff() {
    assert_eq!(levenshtein("cat", "bat"), 1);
}

#[test]
fn levenshtein_empty() {
    assert_eq!(levenshtein("", "abc"), 3);
}

#[test]
fn similar_names_found() {
    let names = vec!["raider1".into(), "raider2".into(), "alice".into()];
    assert!(has_similar_usernames(&names, 2));
}

#[test]
fn similar_names_not_found() {
    let names = vec!["alice".into(), "bob".into(), "charlie".into()];
    assert!(!has_similar_usernames(&names, 1));
}

#[test]
fn clustered_creation() {
    assert!(are_creations_clustered(&[1000, 1500, 2000], 3600));
    assert!(!are_creations_clustered(&[1000, 100000], 3600));
}

#[test]
fn raid_analysis_scoring() {
    let joins = vec![
        JoinInfo {
            username: "raid1".into(),
            has_avatar: false,
            account_created_timestamp: 1000,
        },
        JoinInfo {
            username: "raid2".into(),
            has_avatar: false,
            account_created_timestamp: 1500,
        },
    ];
    let analysis = analyze_joins(&joins, 2, 3600);
    assert!(analysis.score >= 60);
}

#[test]
fn alt_detection_similar_name() {
    let bans = vec![BannedUserInfo {
        username: "raider".into(),
        account_created_timestamp: 5000,
    }];
    let result = check_alt_account("ra1der", 99999, &bans, 2, 3600);
    assert!(result.similar_to_banned.is_some());
}

#[test]
fn alt_detection_no_match() {
    let bans = vec![BannedUserInfo {
        username: "bob".into(),
        account_created_timestamp: 5000,
    }];
    let result = check_alt_account("alice", 99999, &bans, 1, 3600);
    assert!(!result.is_suspicious());
}

#[test]
fn suspicious_account_young() {
    let now = chrono::Utc::now().timestamp();
    assert!(is_account_suspicious(now - 3600, 86400));
}

#[test]
fn suspicious_account_old() {
    let now = chrono::Utc::now().timestamp();
    assert!(!is_account_suspicious(now - 100000, 86400));
}

#[test]
fn raid_analysis_single_join_no_raid() {
    let joins = vec![JoinInfo {
        username: "solo".into(),
        has_avatar: true,
        account_created_timestamp: 1000,
    }];
    assert_eq!(analyze_joins(&joins, 2, 3600).score, 0);
}

#[test]
fn similar_names_single_name() {
    assert!(!has_similar_usernames(&["only".into()], 2));
}

#[test]
fn alt_detection_creation_near() {
    let bans = vec![BannedUserInfo {
        username: "zzzzz".into(),
        account_created_timestamp: 5000,
    }];
    let result = check_alt_account("completely_different", 5500, &bans, 1, 3600);
    assert!(result.creation_near_banned.is_some());
}

#[test]
fn suspicious_account_future_timestamp() {
    let future = chrono::Utc::now().timestamp() + 3600;
    assert!(is_account_suspicious(future, 86400));
}

#[test]
fn alt_detection_breaks_when_both_conditions_on_same_ban() {
    // Couvre le `break` (line 152-153) : quand un seul ban satisfait les 2
    // conditions (similaire + creation proche), on sort de la boucle sans
    // visiter les bans suivants.
    let bans = vec![
        BannedUserInfo {
            username: "raider1".into(),
            account_created_timestamp: 5000,
        },
        BannedUserInfo {
            username: "other_completely_different_name".into(),
            account_created_timestamp: 999_999,
        },
    ];
    let result = check_alt_account("raider2", 5100, &bans, 2, 3600);
    assert_eq!(result.similar_to_banned, Some("raider1".into()));
    assert_eq!(result.creation_near_banned, Some("raider1".into()));
}

#[test]
fn alt_detection_similar_and_creation_from_different_bans() {
    // Cas ou les 2 conditions sont satisfaites mais par des bans differents
    // (pas de break apres la 1re iteration).
    let bans = vec![
        BannedUserInfo {
            username: "raider".into(),
            account_created_timestamp: 999_999,
        },
        BannedUserInfo {
            username: "total_diff".into(),
            account_created_timestamp: 5100,
        },
    ];
    let result = check_alt_account("ra1der", 5000, &bans, 2, 3600);
    assert_eq!(result.similar_to_banned, Some("raider".into()));
    assert_eq!(result.creation_near_banned, Some("total_diff".into()));
}

// ── Couverture des branches restantes ──

#[test]
fn levenshtein_a_non_empty_b_empty() {
    // Couvre la branche `if b_len == 0 { return a_len; }`.
    assert_eq!(levenshtein("abc", ""), 3);
    assert_eq!(levenshtein("hello", ""), 5);
}

#[test]
fn has_similar_usernames_caps_to_50() {
    // Couvre la branche `names.len() > 50 { &names[..50] }`.
    let mut names: Vec<String> = (0..60).map(|i| format!("user{i}")).collect();
    // Ajouter deux doublons dans les 50 premiers pour forcer match.
    names[5] = "duplicate".into();
    names[10] = "duplicate".into();
    assert!(has_similar_usernames(&names, 0));
}

#[test]
fn has_similar_usernames_caps_ignores_items_above_50() {
    // Les doublons sont au-dela de 50 → pas de match (capped).
    let mut names: Vec<String> = (0..60).map(|i| format!("uniq{i}")).collect();
    names[55] = "same".into();
    names[56] = "same".into();
    assert!(!has_similar_usernames(&names, 0));
}

#[test]
fn are_creations_clustered_less_than_two_returns_false() {
    // Couvre `timestamps.len() < 2 → return false`.
    assert!(!are_creations_clustered(&[], 3600));
    assert!(!are_creations_clustered(&[1000], 3600));
}

#[test]
fn analyze_joins_no_raid_indicator_score_zero() {
    // Couvre les branches `score += X` quand la condition est false :
    // deux joins tres espaces, avatars differents, noms differents → score=0.
    let joins = vec![
        JoinInfo {
            username: "alice".into(),
            has_avatar: true,
            account_created_timestamp: 0,
        },
        JoinInfo {
            username: "zebra".into(),
            has_avatar: true,
            account_created_timestamp: 999_999_999,
        },
    ];
    let analysis = analyze_joins(&joins, 0, 60);
    assert_eq!(analysis.score, 0);
    assert!(!analysis.similar_names);
    assert!(!analysis.high_default_avatar_ratio);
    assert!(!analysis.clustered_creation);
}

// ── Politique auto-vs-suggest (mode hybride) ──

#[test]
fn raid_mode_from_config_parsing() {
    assert_eq!(RaidMode::from_config("auto"), RaidMode::Auto);
    assert_eq!(RaidMode::from_config("suggest"), RaidMode::Suggest);
    assert_eq!(RaidMode::from_config("hybrid"), RaidMode::Hybrid);
    // Valeurs inconnues / vides -> Hybrid (defaut).
    assert_eq!(RaidMode::from_config(""), RaidMode::Hybrid);
    assert_eq!(RaidMode::from_config("bogus"), RaidMode::Hybrid);
}

#[test]
fn response_mode_auto_always_auto() {
    // En mode Auto, score et velocity n'importent pas.
    assert_eq!(
        raid_response_mode(0, false, RaidMode::Auto, 85),
        RaidResponseMode::Auto
    );
    assert_eq!(
        raid_response_mode(100, true, RaidMode::Auto, 85),
        RaidResponseMode::Auto
    );
}

#[test]
fn response_mode_suggest_always_suggest() {
    // En mode Suggest, meme un raid massif reste une suggestion.
    assert_eq!(
        raid_response_mode(100, true, RaidMode::Suggest, 85),
        RaidResponseMode::Suggest
    );
    assert_eq!(
        raid_response_mode(0, false, RaidMode::Suggest, 85),
        RaidResponseMode::Suggest
    );
}

#[test]
fn response_mode_hybrid_velocity_always_auto() {
    // Flood de vitesse -> auto meme si le score est bas.
    // Branche desormais atteignable en prod : le bot propage son signal
    // `is_velocity_raid` jusqu'a `raid_response_mode` via l'API (BUG #1).
    assert_eq!(
        raid_response_mode(0, true, RaidMode::Hybrid, 85),
        RaidResponseMode::Auto
    );
}

#[test]
fn response_mode_hybrid_score_boundary() {
    // Juste sous le seuil -> suggestion.
    assert_eq!(
        raid_response_mode(84, false, RaidMode::Hybrid, 85),
        RaidResponseMode::Suggest
    );
    // Pile au seuil -> auto (>=).
    assert_eq!(
        raid_response_mode(85, false, RaidMode::Hybrid, 85),
        RaidResponseMode::Auto
    );
    // Au-dessus -> auto.
    assert_eq!(
        raid_response_mode(90, false, RaidMode::Hybrid, 85),
        RaidResponseMode::Auto
    );
}
