use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::nexus::adapters::inbound::http::dto::game::templates::GameTemplateDto;
use crate::nexus::adapters::inbound::http::handlers::ApiError;
use crate::nexus::bootstrap::AppState;

/// GET /api/games/{guild_id}/templates — liste filtree par allowed_templates.
pub async fn list_templates_for_guild(
    State(state): State<AppState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<GameTemplateDto>>, ApiError> {
    let list = state.game_templates_uc.list_for_guild(&guild_id).await?;
    Ok(Json(list.into_iter().map(GameTemplateDto::from).collect()))
}

/// GET /api/games/templates/{id}
pub async fn get_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GameTemplateDto>, ApiError> {
    let t = state.game_templates_uc.get(id).await?;
    Ok(Json(t.into()))
}
