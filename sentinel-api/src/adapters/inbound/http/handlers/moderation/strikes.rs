//! Handlers strikes — adaptateur HTTP mince sur `ModerationState.strikes_uc`.
//!
//! Restaure la surface HTTP supprimee lors d'un nettoyage trop large : le
//! metier (`ManageStrikesUseCase`) et les DTO avaient survecu, seuls les
//! handlers/routes manquaient. Aucune logique ici, tout passe par le use-case.

use crate::adapters::inbound::http::dto::moderation::strikes::AddStrikeDto;
use crate::adapters::inbound::http::dto::moderation::strikes::SaveStrikeConfigDto;
use crate::adapters::inbound::http::dto::moderation::strikes::StrikeConfigDto;
use crate::adapters::inbound::http::dto::moderation::strikes::StrikeResultDto;
use crate::adapters::inbound::http::dto::moderation::strikes::UserStrikeDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::bootstrap::state::ModerationState;
use axum::extract::State;
use axum::Extension;
use axum::Json;

/// GET /api/strikes/config/{guild_id}
pub async fn get_config(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<StrikeConfigDto>, ApiError> {
    let config = state.strikes_uc.get_config(&guild_id).await?;
    Ok(single_dto(config))
}

/// PUT /api/strikes/config/{guild_id}
pub async fn save_config(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<SaveStrikeConfigDto>,
) -> Result<Json<StrikeConfigDto>, ApiError> {
    // Config des seuils d'escalation = admin (pas moderator).
    let command = dto.into_command(guild_id.into());
    let config = state.strikes_uc.save_config(command).await?;
    Ok(single_dto(config))
}

/// GET /api/strikes/{guild_id}/{user_id}
pub async fn get_active_strikes(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<Vec<UserStrikeDto>>, ApiError> {
    let strikes = state
        .strikes_uc
        .get_active_strikes(&guild_id, &user_id)
        .await?;
    Ok(map_to_dtos(strikes))
}

/// POST /api/strikes
pub async fn add_strike(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<AddStrikeDto>,
) -> Result<Json<StrikeResultDto>, ApiError> {
    let (command, (guild_id, user_id)) = crate::capture_and_into!(dto, guild_id, user_id);
    let result = state.strikes_uc.add_strike(command).await?;

    let active_count = result.active_count;
    let escalation_action = result.escalation_action.clone();
    let escalation_duration = result.escalation_duration;

    state.broadcaster.broadcast(
        "strike_added",
        serde_json::json!({
            "guild_id": guild_id,
            "user_id": user_id,
            "active_count": active_count,
            "escalation_action": escalation_action,
            "escalation_duration": escalation_duration,
        }),
    );

    Ok(single_dto(result))
}

/// DELETE /api/strikes/{guild_id}/{user_id}
pub async fn reset_strikes(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Gate user : moderator+ requis pour reset les strikes d'un user.
    state.strikes_uc.reset_strikes(&guild_id, &user_id).await?;
    Ok(ok_response())
}
