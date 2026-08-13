use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::State;
use axum::Json;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::dto::moderation::rules::CreateRuleDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::rules::RuleResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::bootstrap::state::ModerationState;

pub async fn get_rules(
    State(state): State<ModerationState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<RuleResponseDto>>, ApiError> {
    let rules = state.rules_uc.get_rules(&guild_id).await?;
    Ok(map_to_dtos(rules))
}

pub async fn create_rule(
    State(state): State<ModerationState>,
    Json(dto): Json<CreateRuleDto>,
) -> Result<Json<RuleResponseDto>, ApiError> {
    // Guild fourni dans le body (pas dans le path) : lookup explicite via
    // check_role_for_guild. Pass-through si pas de RoleContext (appel bot).
    let command = dto.into();
    let rule = state.rules_uc.create_or_update_rule(command).await?;
    Ok(single_dto(rule))
}

pub async fn delete_rule(
    State(state): State<ModerationState>,
    Path((guild_id, rule_id)): Path<(String, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.rules_uc.delete_rule(&guild_id, rule_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
