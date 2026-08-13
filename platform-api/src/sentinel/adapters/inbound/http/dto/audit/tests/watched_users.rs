use super::*;
use chrono::TimeZone;
use chrono::Utc;
use platform_core::sentinel::domain::entities::audit::watched_user::WatchedUser;
use platform_core::sentinel::ports::inbound::audit::manage_watched_users::UserDossier;
fn sample_user(last: Option<chrono::DateTime<Utc>>) -> WatchedUser {
    WatchedUser {
        user_id: "u".into(),
        username: "alice".into(),
        guild_id: "g".into(),
        guild_name: "Guild".into(),
        risk_level: "high".into(),
        total_warns: 1,
        total_mutes: 2,
        total_bans: 0,
        last_incident_at: last,
        security_events_count: 3,
        first_seen_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    }
}

#[test]
fn from_watched_user_preserves_fields_and_formats_dates() {
    let last = Utc.with_ymd_and_hms(2024, 6, 15, 12, 30, 0).unwrap();
    let dto = WatchedUserResponseDto::from(sample_user(Some(last)));
    assert_eq!(dto.user_id, "u".into());
    assert_eq!(dto.username, "alice");
    assert_eq!(dto.risk_level, "high");
    assert_eq!(dto.total_mutes, 2);
    assert_eq!(dto.security_events_count, 3);
    assert_eq!(
        dto.last_incident_at.as_deref(),
        Some("2024-06-15T12:30:00+00:00")
    );
    assert!(dto.first_seen_at.starts_with("2024-01-01T"));
}

#[test]
fn from_watched_user_with_no_last_incident() {
    let dto = WatchedUserResponseDto::from(sample_user(None));
    assert!(dto.last_incident_at.is_none());
}

#[test]
fn from_user_dossier_maps_empty_collections() {
    let dossier = UserDossier {
        user: sample_user(None),
        infractions: vec![],
        moderation_actions: vec![],
        security_events: vec![],
        notes: vec![],
    };
    let dto = UserDossierResponseDto::from(dossier);
    assert_eq!(dto.user.user_id, "u".into());
    assert!(dto.infractions.is_empty());
    assert!(dto.moderation_actions.is_empty());
    assert!(dto.security_events.is_empty());
    assert!(dto.notes.is_empty());
}
