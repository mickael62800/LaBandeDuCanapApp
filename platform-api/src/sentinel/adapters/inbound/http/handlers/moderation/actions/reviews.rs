use super::*;

/// MOD #3 — POST /api/moderation/review
#[derive(Debug, serde::Deserialize)]
pub struct AddReviewDto {
    pub action_id: String,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReviewQueueEntryDto {
    pub id: String,
    pub action_id: String,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    pub reason: Option<String>,
    pub status: String,
    pub reviewer_id: Option<String>,
    pub reviewer_name: Option<String>,
    pub reviewer_notes: Option<String>,
    pub added_at: String,
    pub resolved_at: Option<String>,
    // Enrichissement : infos de l'action liee
    pub action_type: Option<String>,
    pub target_name: Option<String>,
    pub action_reason: Option<String>,
}

pub(crate) fn review_entry_to_dto(
    e: platform_core::sentinel::ports::outbound::moderation::review_repository::ReviewEntry,
) -> ReviewQueueEntryDto {
    ReviewQueueEntryDto {
        id: e.id.to_string(),
        action_id: e.action_id.to_string(),
        guild_id: e.guild_id,
        added_by: e.added_by,
        added_by_name: e.added_by_name,
        reason: e.reason,
        status: e.status,
        reviewer_id: e.reviewer_id,
        reviewer_name: e.reviewer_name,
        reviewer_notes: e.reviewer_notes,
        added_at: e.added_at.to_rfc3339(),
        resolved_at: e.resolved_at.map(|d| d.to_rfc3339()),
        action_type: e.action_type,
        target_name: e.target_name,
        action_reason: e.action_reason,
    }
}

pub async fn add_review(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<AddReviewDto>,
) -> Result<Json<ReviewQueueEntryDto>, ApiError> {
    let action_uuid = validation::parse_uuid("action_id", &dto.action_id).map_err(ApiError)?;
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("added_by", &dto.added_by).map_err(ApiError)?;

    let reason = dto.reason.as_deref().map(
        platform_core::sentinel::domain::entities::moderation::review::manual::truncate_review_text,
    );

    let entry = state
        .review_repo
        .add(
            action_uuid,
            &dto.guild_id,
            &dto.added_by,
            &dto.added_by_name,
            reason.as_deref(),
        )
        .await?;

    Ok(Json(review_entry_to_dto(entry)))
}

/// MOD #3 — GET /api/moderation/review/{guild_id}/pending
///
/// Liste les reviews en attente pour une guild, enrichies avec les infos de
/// l'action de moderation liee dans `audit_logs`.
pub async fn list_pending_reviews(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<ReviewQueueEntryDto>>, ApiError> {
    let entries = state.review_repo.list_pending(&guild_id).await?;
    Ok(Json(entries.into_iter().map(review_entry_to_dto).collect()))
}

/// MOD #3 — PATCH /api/moderation/review/{id}/resolve
#[derive(Debug, serde::Deserialize)]
pub struct ResolveReviewDto {
    pub status: String,
    pub reviewer_id: String,
    pub reviewer_name: String,
    #[serde(default)]
    pub reviewer_notes: Option<String>,
}

pub async fn resolve_review(
    State(state): State<ModerationState>,
    // TODO(secu) : aucun gate par role ici. La protection vient uniquement des
    // middlewares du routeur (auth Bearer + superadmin + guild_auth). Le
    // controle fin � Moderator+ sur CETTE guilde � reste a implementer � il
    // existait sous forme d'un `if user.is_some() {}` vide, qui ne verifiait
    // rien tout en donnant l'impression du contraire.
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<ResolveReviewDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let review_uuid = validation::parse_uuid("id", &id).map_err(ApiError)?;

    if !platform_core::sentinel::domain::entities::moderation::review::manual::is_valid_review_status(
        &dto.status,
    ) {
        return Err(ApiError(
            platform_core::sentinel::domain::errors::DomainError::ValidationError(
                "status doit etre approved/rejected/changed".into(),
            ),
        ));
    }
    validation::validate_discord_id("reviewer_id", &dto.reviewer_id).map_err(ApiError)?;
    let notes = dto.reviewer_notes.as_deref().map(
        platform_core::sentinel::domain::entities::moderation::review::manual::truncate_review_text,
    );

    let resolved = state
        .review_repo
        .resolve(
            review_uuid,
            &dto.reviewer_id,
            &dto.reviewer_name,
            notes.as_deref(),
            &dto.status,
        )
        .await?;

    if !resolved {
        return Err(ApiError(
            platform_core::sentinel::domain::errors::DomainError::NotFound(
                "review introuvable ou deja resolue".into(),
            ),
        ));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}
