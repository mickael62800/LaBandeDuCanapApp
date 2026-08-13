use super::*;

#[derive(Debug, Deserialize)]
pub struct DetectionQuery {
    /// Defaut 50, max 200.
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// Optionnel : filtre par utilisateur.
    pub user_id: Option<String>,
}

/// GET /api/automod/{guild_id}/detections
pub async fn list_detections(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<DetectionQuery>,
) -> Result<Json<Vec<InfractionResponseDto>>, ApiError> {
    // Filtre `action = "detection"` : seules les detections automod, pas
    // les actions de moderation (warn/mute/ban...).
    let filters = InfractionFilters {
        user_id: params.user_id,
        action: Some("detection".to_string()),
        limit: normalize_limit(params.limit, 50, 200),
        offset: normalize_offset(params.offset),
    };

    let detections = state
        .infractions_uc
        .list_infractions(&guild_id, filters)
        .await?;
    Ok(map_to_dtos(detections))
}

#[derive(Debug, Deserialize)]
pub struct ListReviewsQuery {
    pub limit: Option<i64>,
    /// Si true, inclut les reviews resolues. Default false (pending only).
    pub include_resolved: Option<bool>,
}

/// GET /api/automod/{guild_id}/reviews
pub async fn list_reviews(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<ListReviewsQuery>,
) -> Result<Json<Vec<AutomodReviewDto>>, ApiError> {
    let limit =
        crate::sentinel::adapters::inbound::http::helpers::normalize_in(params.limit, 100, 1, 500);
    let reviews = if params.include_resolved.unwrap_or(false) {
        state
            .automod_reviews_uc
            .list_recent(&guild_id, limit)
            .await?
    } else {
        state
            .automod_reviews_uc
            .list_pending(&guild_id, limit)
            .await?
    };
    // Enrichit chaque carte avec son salon de discussion (si ouvert) pour le web.
    let mut dtos: Vec<AutomodReviewDto> = Vec::with_capacity(reviews.len());
    for r in reviews {
        let rid = r.id;
        let mut dto: AutomodReviewDto = r.into();
        if let Ok(Some(d)) = state.automod_reviews_uc.get_discussion(rid).await {
            dto.discussion_channel_id = Some(d.channel_id);
        }
        dtos.push(dto);
    }
    Ok(Json(dtos))
}

/// GET /api/automod/reviews/{review_id}
pub async fn get_review(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
) -> Result<Json<AutomodReviewDto>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    match state.automod_reviews_uc.get(id).await? {
        Some(r) => {
            let rid = r.id;
            let mut dto: AutomodReviewDto = r.into();
            if let Ok(Some(d)) = state.automod_reviews_uc.get_discussion(rid).await {
                dto.discussion_channel_id = Some(d.channel_id);
            }
            Ok(Json(dto))
        }
        None => Err(ApiError::from(DomainError::NotFound(format!(
            "review {review_id} introuvable"
        )))),
    }
}

/// GET /api/automod/reviews/{review_id}/votes
pub async fn list_review_votes(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path(review_id): Path<String>,
) -> Result<Json<Vec<ReviewVoteDto>>, ApiError> {
    let id = validation::parse_uuid("review_id", &review_id).map_err(ApiError)?;
    let votes = state.automod_reviews_uc.list_votes(id).await?;
    Ok(Json(votes.into_iter().map(Into::into).collect()))
}

/// GET /api/automod/{guild_id}/reviews/by-message/{message_id}
/// Retrouve la review associee a un message Discord (pour retrouver le
/// review_id depuis une carte 1-clic dont les boutons ne le portent pas).
pub async fn find_review_by_message(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, message_id)): Path<(String, String)>,
) -> Result<Json<Option<AutomodReviewDto>>, ApiError> {
    let review = state
        .automod_reviews_uc
        .find_by_message_id(&guild_id, &message_id)
        .await?;
    Ok(Json(review.map(Into::into)))
}
