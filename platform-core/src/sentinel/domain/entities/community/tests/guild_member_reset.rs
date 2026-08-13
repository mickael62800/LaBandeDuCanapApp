use super::*;

#[test]
fn discord_list_cap_is_1000() {
    // Limite Discord documentee : max 1000 membres par appel API.
    assert_eq!(DISCORD_LIST_MEMBERS_CAP, 1000);
}

#[test]
fn members_cache_ttl_is_10_minutes() {
    assert_eq!(MEMBERS_CACHE_TTL_SECS, 600);
}

#[test]
fn channels_cache_ttl_is_10_minutes() {
    assert_eq!(CHANNELS_CACHE_TTL_SECS, 600);
}

#[test]
fn reset_tables_has_eight_entries() {
    // Regle metier : reset_member touche exactement 8 tables (5 moderation/
    // surveillance + activity_log + user_stats + voice_sessions).
    assert_eq!(MEMBER_RESET_TABLES.len(), 8);
}

#[test]
fn reset_tables_include_core_moderation() {
    let tables: Vec<&str> = MEMBER_RESET_TABLES.iter().map(|t| t.sql_table).collect();
    assert!(tables.contains(&"infractions"));
    assert!(!tables.contains(&"audit_logs"));
    assert!(tables.contains(&"user_strikes"));
    assert!(tables.contains(&"user_notes"));
    assert!(tables.contains(&"manual_watched_users"));
    assert!(tables.contains(&"sanction_reminders"));
}

#[test]
fn reset_tables_use_target_id_for_reminders() {
    // Les rappels utilisent target_id (la personne moderee), pas user_id.
    let reminders = MEMBER_RESET_TABLES
        .iter()
        .find(|t| t.sql_table == "sanction_reminders")
        .unwrap();
    assert_eq!(reminders.user_column, "target_id");
}

#[test]
fn reset_tables_use_user_id_for_others() {
    for t in MEMBER_RESET_TABLES {
        if t.sql_table != "sanction_reminders" {
            assert_eq!(
                t.user_column, "user_id",
                "table {} should use user_id",
                t.sql_table
            );
        }
    }
}

#[test]
fn reset_tables_response_keys_are_unique() {
    use std::collections::HashSet;
    let keys: HashSet<&str> = MEMBER_RESET_TABLES.iter().map(|t| t.response_key).collect();
    assert_eq!(keys.len(), MEMBER_RESET_TABLES.len());
}
