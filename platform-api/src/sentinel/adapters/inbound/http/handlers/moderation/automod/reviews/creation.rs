use super::*;

#[derive(Debug, Deserialize)]
pub struct CreateReviewBody {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub user_id: UserId,
    pub user_name: String,
    pub content_preview: String,
    pub suggested_action: String,
    pub score: f64,
    pub reason: String,
    pub flags: Option<serde_json::Value>,
    /// Si fourni (RFC3339), ouvre la review en mode VOTE avec cette echeance.
    pub voting_deadline: Option<String>,
    /// Si true, agrege l'incident dans la carte 'voting' ouverte du meme
    /// utilisateur (anti-flood). Default false (comportement historique).
    pub aggregate: Option<bool>,
    /// Fenetre d'inactivite (minutes) au-dela de laquelle on n'agrege plus dans
    /// une carte existante. Default 60 ; 0 = pas de limite.
    pub aggregate_window_minutes: Option<i64>,
    /// `true` si l'auto-protection sévère a DÉJÀ journalisé une sanction de
    /// membre pour cet incident (mute auto). La finalisation de la carte NE
    /// re-journalise alors PAS la sanction (anti double-strike, cf. C1).
    #[serde(default)]
    pub already_sanctioned: bool,
}

/// POST /api/automod/reviews
///
/// Endpoint d'ingestion : appele par le bot juste apres avoir poste la
/// carte de review dans le channel Discord. Permet au web de lister les
/// reviews en attente.
pub async fn create_review(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Json(body): Json<CreateReviewBody>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let suggested = SuggestedAction::from_str(&body.suggested_action).ok_or_else(|| {
        ApiError::from(DomainError::ValidationError(format!(
            "suggested_action invalide : {}",
            body.suggested_action
        )))
    })?;

    let (review, merged) = state
        .automod_reviews_uc
        .create_or_merge(
            NewAutomodReview {
                guild_id: body.guild_id.clone(),
                channel_id: body.channel_id,
                message_id: body.message_id,
                user_id: body.user_id.clone(),
                user_name: body.user_name,
                content_preview: body.content_preview,
                suggested_action: suggested,
                score: body.score,
                reason: body.reason,
                flags: body.flags.unwrap_or(serde_json::json!({})),
                voting_deadline: body
                    .voting_deadline
                    .as_deref()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&chrono::Utc)),
                sanction_logged: body.already_sanctioned,
            },
            body.aggregate.unwrap_or(false),
            body.aggregate_window_minutes.unwrap_or(60),
        )
        .await?;

    // Notification web : creation OU mise a jour (agregation) d'une review.
    state.broadcaster.broadcast(
        if merged {
            "automod_review_updated"
        } else {
            "automod_review_created"
        },
        serde_json::json!({
            "review_id": review.id.to_string(),
            "guild_id": &review.guild_id,
            "user_id": &review.user_id,
            "merged": merged,
        }),
    );

    let mut dto: AutomodReviewDto = review.into();
    dto.merged = merged;
    Ok(Json(dto))
}
