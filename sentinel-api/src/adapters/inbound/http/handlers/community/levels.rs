use crate::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use axum::extract::Query;
use axum::extract::State;
use axum::Json;

use crate::adapters::inbound::http::dto::community::levels::AddXpDto;
use crate::adapters::inbound::http::dto::community::levels::AddXpResponseDto;
use crate::adapters::inbound::http::dto::community::levels::LevelLeaderboardParams;
use crate::adapters::inbound::http::dto::community::levels::ResetUserXpDto;
use crate::adapters::inbound::http::dto::community::levels::SetUserXpDto;
use crate::adapters::inbound::http::dto::community::levels::UserLevelDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::normalize_limit;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::bootstrap::state::CommunityState;
use sentinel_core::domain::entities::community::level::XpSource;
use sentinel_core::ports::inbound::community::manage_levels::AddXpCommand;
use sentinel_core::ports::inbound::community::manage_levels::ResetTarget;
use sentinel_core::ports::inbound::community::manage_levels::SetUserXpCommand;

pub async fn add_xp(
    State(state): State<CommunityState>,
    Json(dto): Json<AddXpDto>,
) -> Result<Json<AddXpResponseDto>, ApiError> {
    // L'attribution d'XP est une operation du BOT (Bearer API_KEY -> Internal,
    // bypass). Un appelant web ne doit pas pouvoir s'auto-crediter de l'XP :
    // reserve admin+ (avant, AUCUNE garde -> auto-attribution).
    let guild_id = dto.guild_id.clone();
    let user_id = dto.user_id.clone();
    let amount = dto.amount;
    let source = XpSource::from_str(&dto.source);

    let result = state
        .levels_uc
        .add_xp(AddXpCommand {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            amount: dto.amount,
            source,
        })
        .await?;

    state.broadcaster.broadcast(
        "xp_gained",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "amount": amount,
            "source": source.as_str(),
        }),
    );

    Ok(single_dto(result))
}

pub async fn get_user_level(
    State(state): State<CommunityState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<UserLevelDto>, ApiError> {
    let level = state.levels_uc.get_user_level(&guild_id, &user_id).await?;
    Ok(single_dto(level))
}

pub async fn get_leaderboard(
    State(state): State<CommunityState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<LevelLeaderboardParams>,
) -> Result<Json<Vec<UserLevelDto>>, ApiError> {
    let limit = normalize_limit(params.limit, 25, 100);
    let levels = match params.source.as_deref() {
        Some("text") => {
            state
                .levels_uc
                .get_leaderboard_by_source(&guild_id, XpSource::Text, limit)
                .await?
        }
        Some("voice") => {
            state
                .levels_uc
                .get_leaderboard_by_source(&guild_id, XpSource::Voice, limit)
                .await?
        }
        _ => state.levels_uc.get_leaderboard(&guild_id, limit).await?,
    };
    Ok(map_to_dtos(levels))
}

pub async fn set_user_xp(
    State(state): State<CommunityState>,
    Json(dto): Json<SetUserXpDto>,
) -> Result<Json<UserLevelDto>, ApiError> {
    let guild_id = dto.guild_id.clone();
    let user_id = dto.user_id.clone();
    let user = state
        .levels_uc
        .set_user_xp(SetUserXpCommand {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            xp_text: dto.xp_text,
            xp_voice: dto.xp_voice,
        })
        .await?;
    state.broadcaster.broadcast(
        "xp_admin_set",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "xp": user.xp,
            "level": user.level,
        }),
    );
    Ok(single_dto(user))
}

pub async fn reset_user_xp(
    State(state): State<CommunityState>,
    Json(dto): Json<ResetUserXpDto>,
) -> Result<Json<UserLevelDto>, ApiError> {
    let target = match dto.target.as_str() {
        "text" => ResetTarget::Text,
        "voice" => ResetTarget::Voice,
        "all" => ResetTarget::All,
        other => {
            return Err(ApiError(
                sentinel_core::domain::errors::DomainError::ValidationError(format!(
                    "target invalide: {other} (attendu: all/text/voice)"
                )),
            ));
        }
    };
    let guild_id = dto.guild_id.clone();
    let user_id = dto.user_id.clone();
    let user = state
        .levels_uc
        .reset_user_xp(&guild_id, &user_id, target)
        .await?;
    state.broadcaster.broadcast(
        "xp_admin_reset",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "target": dto.target,
        }),
    );
    Ok(single_dto(user))
}

#[cfg(test)]
#[path = "tests/levels.rs"]
mod tests;
