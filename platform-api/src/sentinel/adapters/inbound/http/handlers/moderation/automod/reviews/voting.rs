use super::lifecycle::effective_facts;
use super::*;

#[derive(Debug, Deserialize)]
pub struct CastVoteBody {
    pub voter_id: String,
    pub voter_name: String,
    /// "warn" | "delete" | "mute" | "ban" | "ignore".
    pub vote_action: String,
    // Faits Discord du votant ; la regle is_moderator est appliquee cote domaine.
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub has_moderate_members: bool,
    #[serde(default)]
    pub has_manage_messages: bool,
    #[serde(default)]
    pub has_mod_role: bool,
}

/// POST /api/automod/reviews/{review_id}/vote
pub async fn vote_review(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
    Json(body): Json<CastVoteBody>,
) -> Result<Json<Vec<ReviewVoteDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    // Bot (de confiance) : faits gateway du body. Web : faits du role REEL
    // (le body est ignore) -> `is_moderator` exige un vrai Moderator+.
    let body_facts = ModeratorFacts {
        is_admin: body.is_admin,
        has_moderate_members: body.has_moderate_members,
        has_manage_messages: body.has_manage_messages,
        has_mod_role: body.has_mod_role,
        has_admin_role: false,
    };
    let requester = effective_facts(&state, &user, id, Some(body_facts))
        .await?
        .unwrap_or_default();
    let votes = state
        .automod_reviews_uc
        .cast_vote(
            platform_core::sentinel::ports::inbound::moderation::manage_automod_reviews::CastVoteCommand {
                review_id: id,
                voter_id: body.voter_id.clone(),
                voter_name: body.voter_name.clone(),
                vote_action: body.vote_action.clone(),
                requester,
            },
        )
        .await?;
    state.broadcaster.broadcast(
        "automod_review_voted",
        serde_json::json!({ "review_id": review_id, "votes": votes.len() }),
    );
    Ok(Json(votes.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
pub struct DecideReviewBody {
    pub quorum: i64,
    /// "ignore" | "clemente" | "severe".
    pub tie_action: String,
}

/// POST /api/automod/reviews/{review_id}/decide
///
/// Cloture le vote (appele par le worker a l'echeance). Depouille et passe
/// la review en 'decided'. Publie `automod_review_decided` pour que le bot
/// edite la carte et revele le bouton admin de finalisation.
pub async fn decide_review(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
    Json(body): Json<DecideReviewBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    // Le clamp 1..100 du quorum est appliqué par le use case (règle métier).
    let quorum = body.quorum.max(0) as usize;
    let (review, tally) = state
        .automod_reviews_uc
        .decide(id, quorum, &body.tie_action)
        .await?;
    broadcast_decision(&state, &review, &tally);
    Ok(Json(review.into()))
}

/// POST /api/automod/internal/jobs/close-votes
pub async fn job_close_expired_votes(
    State(state): State<ModerationState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    use platform_core::sentinel::domain::entities::system::bot_config::{cfg_i64, cfg_str};

    let reviews = state.automod_reviews_uc.list_expired_voting(50).await?;
    let mut processed = 0usize;
    let mut errors = 0usize;
    for review in reviews {
        let config = match state
            .bot_config_repo
            .get_config(&review.guild_id, "automod-bot")
            .await
        {
            Ok(config) => config,
            Err(error) => {
                errors += 1;
                tracing::warn!(review_id = %review.id, %error, "lecture config AutoMod impossible");
                continue;
            }
        };
        let quorum = cfg_i64(&config, "vote_quorum", 3).max(1) as usize;
        let tie_action = cfg_str(&config, "vote_tie_action").unwrap_or("ignore");
        match state
            .automod_reviews_uc
            .decide(review.id, quorum, tie_action)
            .await
        {
            Ok((decided, tally)) => {
                broadcast_decision(&state, &decided, &tally);
                processed += 1;
            }
            Err(error) => {
                errors += 1;
                tracing::warn!(review_id = %review.id, %error, "cloture vote AutoMod impossible");
            }
        }
    }
    Ok(Json(serde_json::json!({
        "job": "automod_close_votes",
        "processed": processed,
        "errors": errors,
    })))
}

fn broadcast_decision(
    state: &ModerationState,
    review: &platform_core::sentinel::domain::entities::moderation::review::automod::AutomodReview,
    tally: &platform_core::sentinel::domain::entities::moderation::review::automod::TallyResult,
) {
    state.broadcaster.broadcast(
        "automod_review_decided",
        serde_json::json!({
            "review_id": review.id.to_string(),
            "action_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "decided_action": &review.decided_action,
            "quorum_met": tally.quorum_met,
            "total_votes": tally.total_votes,
        }),
    );
}
