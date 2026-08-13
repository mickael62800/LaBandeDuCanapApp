//! Tests du service `ManageAutomodReviewsService` — focalisés sur la règle
//! anti conflit d'intérêt (S2) : un acteur ne peut pas voter / finaliser /
//! clore sa propre détection.

use super::*;
use crate::sentinel::domain::entities::moderation::review::automod::AutomodReview;
use crate::sentinel::domain::entities::moderation::review::automod::ModeratorFacts;
use crate::sentinel::domain::entities::moderation::review::automod::ReviewVote;
use std::sync::Mutex;

/// Construit une review minimale ciblant `user_id`, en statut `status`.
fn review_for(user_id: &str, status: &str) -> AutomodReview {
    AutomodReview {
        id: Uuid::new_v4(),
        guild_id: "123".into(),
        channel_id: "456".into(),
        message_id: "789".into(),
        user_id: user_id.into(),
        user_name: "Flagged".to_string(),
        content_preview: "msg".to_string(),
        suggested_action: "warn".to_string(),
        score: 5.0,
        reason: "test".to_string(),
        flags: serde_json::json!({}),
        status: status.to_string(),
        applied_action: None,
        resolved_by_id: None,
        resolved_by_name: None,
        resolved_source: None,
        created_at: chrono::Utc::now(),
        resolved_at: None,
        voting_deadline: None,
        decided_action: None,
        quorum_met: false,
        decided_at: None,
        incident_count: 1,
        cumulative_score: 5.0,
        incidents: serde_json::json!([]),
        sanction_logged: false,
    }
}

/// Mock de repo : renvoie une review fixe pour `get`, et des Ok triviaux pour
/// les méthodes mutantes (qui ne sont atteintes que si la règle S2 passe).
struct MockRepo {
    review: AutomodReview,
    upsert_called: Mutex<bool>,
}

impl MockRepo {
    fn new(review: AutomodReview) -> Self {
        Self {
            review,
            upsert_called: Mutex::new(false),
        }
    }
}

#[async_trait]
impl AutomodReviewRepository for MockRepo {
    async fn create(&self, _r: NewAutomodReview) -> Result<AutomodReview, DomainError> {
        Ok(self.review.clone())
    }
    async fn create_or_merge(
        &self,
        _r: NewAutomodReview,
        _a: bool,
        _w: i64,
    ) -> Result<(AutomodReview, bool), DomainError> {
        Ok((self.review.clone(), false))
    }
    async fn get(&self, _id: Uuid) -> Result<Option<AutomodReview>, DomainError> {
        Ok(Some(self.review.clone()))
    }
    async fn find_by_message_id(
        &self,
        _g: &str,
        _m: &str,
    ) -> Result<Option<AutomodReview>, DomainError> {
        Ok(Some(self.review.clone()))
    }
    async fn list_pending(&self, _g: &str, _l: i64) -> Result<Vec<AutomodReview>, DomainError> {
        Ok(vec![])
    }
    async fn list_recent(&self, _g: &str, _l: i64) -> Result<Vec<AutomodReview>, DomainError> {
        Ok(vec![])
    }
    async fn resolve(
        &self,
        _id: Uuid,
        _a: &str,
        _ri: &str,
        _rn: &str,
        _s: &str,
    ) -> Result<AutomodReview, DomainError> {
        Ok(self.review.clone())
    }
    async fn close_ignored(
        &self,
        _id: Uuid,
        _ai: &str,
        _an: &str,
        _s: &str,
    ) -> Result<AutomodReview, DomainError> {
        Ok(self.review.clone())
    }
    async fn reopen(&self, _id: Uuid, _h: i64) -> Result<AutomodReview, DomainError> {
        Ok(self.review.clone())
    }
    async fn upsert_vote(
        &self,
        _id: Uuid,
        _vi: &str,
        _vn: &str,
        _va: &str,
    ) -> Result<(), DomainError> {
        *self.upsert_called.lock().unwrap() = true;
        Ok(())
    }
    async fn list_votes(&self, _id: Uuid) -> Result<Vec<ReviewVote>, DomainError> {
        Ok(vec![])
    }
    async fn decide(&self, _id: Uuid, _a: &str, _q: bool) -> Result<AutomodReview, DomainError> {
        Ok(self.review.clone())
    }
    async fn list_expired_voting(&self, _l: i64) -> Result<Vec<AutomodReview>, DomainError> {
        Ok(vec![])
    }
    async fn expire_review_cards(
        &self,
        _d: i64,
        _l: i64,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::review::automod::ExpiredReviewCard>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn expire_stale_decided(
        &self,
        _grace_hours: i64,
        _l: i64,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::review::automod::ExpiredReviewCard>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn fp_terminal_reviews(
        &self,
        _g: &str,
        _d: i64,
        _l: i64,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::review::automod::FpTerminalReview>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn find_discussion(
        &self,
        _id: Uuid,
    ) -> Result<
        Option<crate::sentinel::domain::entities::moderation::review::automod::DiscussionChannel>,
        DomainError,
    > {
        Ok(None)
    }
    async fn create_discussion(
        &self,
        _d: crate::sentinel::domain::entities::moderation::review::automod::NewDiscussionChannel,
    ) -> Result<
        (
            crate::sentinel::domain::entities::moderation::review::automod::DiscussionChannel,
            bool,
        ),
        DomainError,
    > {
        Err(DomainError::NotFound("n/a".into()))
    }
    async fn delete_discussion(&self, _id: Uuid) -> Result<(), DomainError> {
        Ok(())
    }
    async fn append_discussion_messages(
        &self,
        _m: &[crate::sentinel::domain::entities::moderation::review::automod::DiscussionMessage],
    ) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn list_discussion_messages(
        &self,
        _id: Uuid,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::moderation::review::automod::DiscussionMessage>,
        DomainError,
    > {
        Ok(vec![])
    }
}

/// Admin / fondateur : pleins pouvoirs (peut passer outre le garde anti-conflit).
fn moderator_facts() -> ModeratorFacts {
    ModeratorFacts {
        is_admin: true,
        has_moderate_members: true,
        has_manage_messages: true,
        has_mod_role: true,
        has_admin_role: true,
    }
}

/// Moderateur ORDINAIRE : peut voter, mais reste soumis au garde anti-conflit
/// (ne peut pas statuer sur sa propre detection).
fn mod_ordinaire_facts() -> ModeratorFacts {
    ModeratorFacts {
        is_admin: false,
        has_moderate_members: true,
        has_manage_messages: true,
        has_mod_role: true,
        has_admin_role: false,
    }
}

#[tokio::test]
async fn cast_vote_rejette_le_propre_dossier_d_un_mod_ordinaire() {
    let repo = Arc::new(MockRepo::new(review_for("flagged_user", "voting")));
    let svc = ManageAutomodReviewsService::new(repo.clone());
    let err = svc
        .cast_vote(CastVoteCommand {
            review_id: Uuid::new_v4(),
            voter_id: "flagged_user".to_string(),
            voter_name: "Flagged".to_string(),
            vote_action: "warn".to_string(),
            requester: mod_ordinaire_facts(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
    // La règle S2 court-circuite AVANT l'écriture du vote.
    assert!(!*repo.upsert_called.lock().unwrap());
}

/// Pleins pouvoirs : un admin/fondateur PEUT agir sur sa propre detection.
#[tokio::test]
async fn cast_vote_admin_peut_agir_sur_son_propre_dossier() {
    let repo = Arc::new(MockRepo::new(review_for("flagged_user", "voting")));
    let svc = ManageAutomodReviewsService::new(repo.clone());
    let res = svc
        .cast_vote(CastVoteCommand {
            review_id: Uuid::new_v4(),
            voter_id: "flagged_user".to_string(),
            voter_name: "Flagged".to_string(),
            vote_action: "warn".to_string(),
            requester: moderator_facts(),
        })
        .await;
    assert!(
        res.is_ok(),
        "un admin doit pouvoir voter sur son propre dossier"
    );
    assert!(*repo.upsert_called.lock().unwrap());
}

/// Pleins pouvoirs : un admin/fondateur PEUT finaliser sa propre detection
/// (c'est le cas du fondateur en back-office web : facts Owner -> bypass).
#[tokio::test]
async fn resolve_admin_peut_finaliser_son_propre_dossier() {
    let repo = Arc::new(MockRepo::new(review_for("flagged_user", "decided")));
    let svc = ManageAutomodReviewsService::new(repo);
    let res = svc
        .resolve(ResolveAutomodReviewCommand {
            review_id: Uuid::new_v4(),
            applied_action: "ban".to_string(),
            resolved_by_id: "flagged_user".to_string(),
            resolved_by_name: "Flagged".to_string(),
            resolved_source: "web".to_string(),
            requester: Some(moderator_facts()),
        })
        .await;
    assert!(
        res.is_ok(),
        "un admin/fondateur doit pouvoir finaliser son propre dossier"
    );
}

#[tokio::test]
async fn cast_vote_autorise_un_autre_moderateur() {
    let repo = Arc::new(MockRepo::new(review_for("flagged_user", "voting")));
    let svc = ManageAutomodReviewsService::new(repo.clone());
    let res = svc
        .cast_vote(CastVoteCommand {
            review_id: Uuid::new_v4(),
            voter_id: "another_mod".to_string(),
            voter_name: "Mod".to_string(),
            vote_action: "warn".to_string(),
            requester: moderator_facts(),
        })
        .await;
    assert!(res.is_ok());
    assert!(*repo.upsert_called.lock().unwrap());
}

#[tokio::test]
async fn resolve_rejette_son_propre_dossier() {
    let repo = Arc::new(MockRepo::new(review_for("flagged_user", "decided")));
    let svc = ManageAutomodReviewsService::new(repo);
    let err = svc
        .resolve(ResolveAutomodReviewCommand {
            review_id: Uuid::new_v4(),
            applied_action: "ban".to_string(),
            resolved_by_id: "flagged_user".to_string(),
            resolved_by_name: "Flagged".to_string(),
            resolved_source: "web".to_string(),
            requester: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn close_ignored_rejette_son_propre_dossier() {
    let repo = Arc::new(MockRepo::new(review_for("flagged_user", "voting")));
    let svc = ManageAutomodReviewsService::new(repo);
    let err = svc
        .close_ignored(CloseIgnoredCommand {
            review_id: Uuid::new_v4(),
            actor_id: "flagged_user".to_string(),
            actor_name: "Flagged".to_string(),
            source: "web".to_string(),
            requester: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}
