use super::*;

// ── CreateNameHistoryDto ──

#[test]
fn create_name_history_dto_deserializes() {
    let raw = r#"{"guild_id":"g","user_id":"u","old_name":"Alice","new_name":"Alicia"}"#;
    let dto: CreateNameHistoryDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.old_name, "Alice");
    assert_eq!(dto.new_name, "Alicia");
}

// ── UpdateStreakDto ──

#[test]
fn update_streak_dto_deserializes() {
    let raw =
        r#"{"streak_current":5,"streak_best":10,"streak_last_day":15,"streak_last_year":2026}"#;
    let dto: UpdateStreakDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.streak_current, 5);
    assert_eq!(dto.streak_best, 10);
    assert_eq!(dto.streak_last_day, 15);
    assert_eq!(dto.streak_last_year, 2026);
}

// ── UpdateTicketSlaDto ──

#[test]
fn update_ticket_sla_dto_all_optional() {
    let dto: UpdateTicketSlaDto = serde_json::from_str(r#"{}"#).unwrap();
    assert!(dto.first_response_at.is_none());
    assert!(dto.resolved_at.is_none());
    assert!(dto.satisfaction_rating.is_none());
}

#[test]
fn update_ticket_sla_dto_partial() {
    let raw = r#"{"first_response_at":"2026-01-01T00:00:00Z","satisfaction_rating":5}"#;
    let dto: UpdateTicketSlaDto = serde_json::from_str(raw).unwrap();
    assert_eq!(
        dto.first_response_at.as_deref(),
        Some("2026-01-01T00:00:00Z")
    );
    assert_eq!(dto.satisfaction_rating, Some(5));
    assert!(dto.resolved_at.is_none());
}

// ── CreateSponsorshipDto ──

#[test]
fn create_sponsorship_dto_deserializes() {
    let raw = r#"{"guild_id":"g","sponsor_id":"s","sponsored_id":"t"}"#;
    let dto: CreateSponsorshipDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.sponsor_id, "s");
    assert_eq!(dto.sponsored_id, "t");
}

// ── CreateTempRoleDto ──

#[test]
fn create_temp_role_dto_deserializes() {
    let raw = r#"{"guild_id":"g","user_id":"u","role_id":"r","expires_at":"2026-12-31T23:59:59Z"}"#;
    let dto: CreateTempRoleDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.role_id, "r".into());
    assert_eq!(dto.expires_at, "2026-12-31T23:59:59Z");
}

// ── CreatePendingActionDto ──

#[test]
fn create_pending_action_minimal() {
    let raw = r#"{
        "guild_id":"g","moderator_id":"m","moderator_name":"Mod",
        "target_id":"t","target_name":"Target",
        "action_type":"ban","reason":"spam"
    }"#;
    let dto: CreatePendingActionDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.action_type, "ban");
    assert_eq!(dto.reason, "spam");
    assert!(dto.gravity.is_none());
    assert!(dto.duration.is_none());
}

#[test]
fn create_pending_action_with_gravity_and_duration() {
    let raw = r#"{
        "guild_id":"g","moderator_id":"m","moderator_name":"Mod",
        "target_id":"t","target_name":"Target",
        "action_type":"mute","reason":"r","gravity":"high","duration":3600
    }"#;
    let dto: CreatePendingActionDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.gravity.as_deref(), Some("high"));
    assert_eq!(dto.duration, Some(3600));
}

// ── ResolvePendingActionDto ──

#[test]
fn resolve_pending_action_dto_deserializes() {
    let raw = r#"{"status":"approved","reviewed_by":"owner1"}"#;
    let dto: ResolvePendingActionDto = serde_json::from_str(raw).unwrap();
    assert_eq!(dto.status, "approved");
    assert_eq!(dto.reviewed_by, "owner1");
}
