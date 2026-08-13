use super::*;

#[test]
fn maps_event_types_to_counters() {
    let counts = vec![
        ("member_join".to_string(), 15u64),
        ("member_leave".to_string(), 3),
        ("member_ban".to_string(), 2),
        ("message_delete".to_string(), 40),
        ("message_delete_bulk".to_string(), 2),
        ("message_edit".to_string(), 18),
        ("role_create".to_string(), 1),
        ("role_update".to_string(), 3),
        ("role_delete".to_string(), 1),
        ("channel_create".to_string(), 1),
        ("channel_delete".to_string(), 0),
        ("voice_join".to_string(), 60),
        ("voice_leave".to_string(), 40),
        ("anomaly_detected".to_string(), 0),
    ];

    let report = WeeklyReport::from_event_counts(counts);

    assert_eq!(report.member_joins, 15);
    assert_eq!(report.member_leaves, 3);
    assert_eq!(report.bans, 2);
    // message_delete + message_delete_bulk cumules.
    assert_eq!(report.messages_deleted, 42);
    assert_eq!(report.messages_edited, 18);
    // role_create + role_update + role_delete cumules.
    assert_eq!(report.role_changes, 5);
    assert_eq!(report.channel_changes, 1);
    // voice_join + voice_leave cumules.
    assert_eq!(report.voice_events, 100);
    assert_eq!(report.anomalies, 0);
}

#[test]
fn ignores_unknown_event_types() {
    let counts = vec![
        ("member_join".to_string(), 5u64),
        ("member_nickname_update".to_string(), 99),
        ("security_raid".to_string(), 7),
    ];

    let report = WeeklyReport::from_event_counts(counts);

    assert_eq!(report.member_joins, 5);
    // Les event_type non mappes n'affectent aucun compteur.
    assert_eq!(
        report,
        WeeklyReport {
            member_joins: 5,
            ..Default::default()
        }
    );
}

#[test]
fn empty_counts_yield_zero_report() {
    let report = WeeklyReport::from_event_counts(Vec::<(String, u64)>::new());
    assert_eq!(report, WeeklyReport::default());
}
