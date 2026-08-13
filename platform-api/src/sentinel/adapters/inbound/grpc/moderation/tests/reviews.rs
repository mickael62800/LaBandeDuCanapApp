//! Tests des fonctions de mapping du `AutomodReviewService`.
//!
//! On teste les conversions pures (entite <-> proto, parsing) qui portent le
//! risque de bug. Le comportement des RPC eux-memes est un passthrough mince
//! vers `ManageAutomodReviewsUseCase` ; la logique de `log_review_sanction`
//! (chemin resolve) est deja couverte par les tests du handler HTTP.

use super::*;
use chrono::TimeZone;

fn sample_review() -> AutomodReview {
    AutomodReview {
        id: uuid::Uuid::nil(),
        guild_id: "g1".into(),
        channel_id: "c1".into(),
        message_id: "m1".into(),
        user_id: "u1".into(),
        user_name: "Alice".into(),
        content_preview: "coucou".into(),
        suggested_action: "warn".into(),
        score: 0.75,
        reason: "spam".into(),
        flags: serde_json::json!({"spam": true}),
        status: "voting".into(),
        applied_action: None,
        resolved_by_id: None,
        resolved_by_name: None,
        resolved_source: None,
        created_at: chrono::Utc.with_ymd_and_hms(2026, 1, 2, 3, 4, 5).unwrap(),
        resolved_at: None,
        voting_deadline: Some(chrono::Utc.with_ymd_and_hms(2026, 1, 3, 0, 0, 0).unwrap()),
        decided_action: None,
        quorum_met: false,
        decided_at: None,
        incident_count: 2,
        cumulative_score: 1.5,
        incidents: serde_json::json!([{"message_id": "m0"}]),
        sanction_logged: false,
    }
}

#[test]
fn review_to_proto_maps_all_fields() {
    let p = review_to_proto(sample_review(), true, Some("disc1".into()));
    assert_eq!(p.id, uuid::Uuid::nil().to_string());
    assert_eq!(p.guild_id, "g1");
    assert_eq!(p.channel_id, "c1");
    assert_eq!(p.message_id, "m1");
    assert_eq!(p.user_id, "u1");
    assert_eq!(p.user_name, "Alice");
    assert_eq!(p.suggested_action, "warn");
    assert_eq!(p.score, 0.75);
    assert_eq!(p.status, "voting");
    assert!(p.applied_action.is_none());
    assert_eq!(p.created_at, "2026-01-02T03:04:05+00:00");
    assert!(p.voting_deadline.is_some());
    assert_eq!(p.incident_count, 2);
    assert_eq!(p.cumulative_score, 1.5);
    assert!(p.merged);
    assert_eq!(p.discussion_channel_id.as_deref(), Some("disc1"));
    // Les champs JSON sont serialises en string.
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&p.flags_json).unwrap(),
        serde_json::json!({"spam": true})
    );
    assert!(p.incidents_json.contains("m0"));
}

#[test]
fn review_to_proto_preserves_none_optionals() {
    let p = review_to_proto(sample_review(), false, None);
    assert!(p.resolved_at.is_none());
    assert!(p.decided_action.is_none());
    assert!(p.discussion_channel_id.is_none());
    assert!(!p.merged);
}

#[test]
fn facts_from_proto_roundtrip() {
    let f = facts_from_proto(proto::ModeratorFacts {
        is_admin: true,
        has_moderate_members: false,
        has_manage_messages: true,
        has_mod_role: true,
        has_admin_role: false,
    });
    assert!(f.is_admin);
    assert!(!f.has_moderate_members);
    assert!(f.has_manage_messages);
    assert!(f.has_mod_role);
    assert!(!f.has_admin_role);
}

#[test]
fn votes_to_proto_maps_list() {
    let mk = |voter_id: &str, voter_name: &str, vote_action: &str| ReviewVote {
        id: uuid::Uuid::nil(),
        review_id: uuid::Uuid::nil(),
        voter_id: voter_id.into(),
        voter_name: voter_name.into(),
        vote_action: vote_action.into(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let votes = vec![mk("v1", "Bob", "ban"), mk("v2", "Carol", "warn")];
    let list = votes_to_proto(votes);
    assert_eq!(list.votes.len(), 2);
    assert_eq!(list.votes[0].voter_id, "v1");
    assert_eq!(list.votes[1].vote_action, "warn");
}

#[test]
fn parse_json_empty_is_empty_object() {
    assert_eq!(
        parse_json_or_empty_object("").unwrap(),
        serde_json::json!({})
    );
    assert_eq!(
        parse_json_or_empty_object("   ").unwrap(),
        serde_json::json!({})
    );
}

#[test]
fn parse_json_valid_and_invalid() {
    assert_eq!(
        parse_json_or_empty_object(r#"{"a":1}"#).unwrap(),
        serde_json::json!({"a": 1})
    );
    let err = parse_json_or_empty_object("{bad").unwrap_err();
    assert_eq!(err.code(), tonic::Code::InvalidArgument);
}

#[test]
fn parse_uuid_rejects_garbage() {
    assert!(parse_uuid("not-a-uuid").is_err());
    assert!(parse_uuid(&uuid::Uuid::nil().to_string()).is_ok());
}
