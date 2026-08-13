use super::*;

/// GET /api/moderation/bans
pub async fn list_bans(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<BansQuery>,
) -> Result<Json<Vec<BanEntryDto>>, ApiError> {
    // Validation
    validation::validate_optional_discord_id("guild_id", &params.guild_id).map_err(ApiError)?;
    validation::validate_pagination(params.limit, params.offset).map_err(ApiError)?;

    let limit =
        crate::sentinel::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500);
    let offset = crate::sentinel::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let bans = state
        .moderation_uc
        .list_bans(params.guild_id.as_deref(), limit, offset)
        .await?;
    Ok(map_to_dtos(bans))
}

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<UserHistoryDto>, ApiError> {
    // Validation

    let history = state.moderation_uc.get_history(&guild_id, &user_id).await?;
    Ok(single_dto(history))
}
