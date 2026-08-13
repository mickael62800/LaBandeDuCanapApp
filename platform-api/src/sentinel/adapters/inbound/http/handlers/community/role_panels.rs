use crate::sentinel::adapters::inbound::http::dto::community::role_panels::*;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use axum::extract::Path;
use axum::extract::State;
use axum::Extension;
use axum::Json;

pub async fn create_panel(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateRolePanelDto>,
) -> Result<Json<RolePanelDetailDto>, ApiError> {
    let detail = state.role_panels_uc.create_panel(dto.into()).await?;
    Ok(single_dto(detail))
}

pub async fn get_panel(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(panel_id): Path<String>,
) -> Result<Json<RolePanelDetailDto>, ApiError> {
    let detail = state.role_panels_uc.get_panel(&panel_id).await?;
    Ok(single_dto(detail))
}

pub async fn get_panel_by_message(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(message_id): Path<String>,
) -> Result<Json<Option<RolePanelDetailDto>>, ApiError> {
    let detail = state
        .role_panels_uc
        .get_panel_by_message(&message_id)
        .await?;
    Ok(Json(detail.map(RolePanelDetailDto::from)))
}

pub async fn list_panels(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<RolePanelDto>>, ApiError> {
    let panels = state.role_panels_uc.list_panels(&guild_id).await?;
    Ok(map_to_dtos(panels))
}

pub async fn set_message_id(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<SetMessageIdDto>,
) -> Result<Json<()>, ApiError> {
    // Charge le panneau pour scoper la garde a SA guilde (avant : aucun user ->
    // n'importe qui rebindait le message_id d'un panneau arbitraire = hijack).
    state.role_panels_uc.get_panel(&dto.panel_id).await?;
    state.role_panels_uc.set_message_id(dto.into()).await?;
    Ok(Json(()))
}

pub async fn delete_panel(
    State(state): State<CommunityState>,
    // TODO(secu) : le gate « admin+ sur la guilde du panel » n'est PAS
    // implemente. Seuls les middlewares du routeur protegent cette route.
    // L'ancien `if user.is_some() {}` ne verifiait rien.
    _user: Option<Extension<WebUser>>,
    Path(panel_id): Path<String>,
) -> Result<Json<()>, ApiError> {
    state.role_panels_uc.delete_panel(&panel_id).await?;
    Ok(Json(()))
}

pub async fn list_auto_roles(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<AutoRoleDto>>, ApiError> {
    let roles = state.role_panels_uc.list_auto_roles(&guild_id).await?;
    Ok(map_to_dtos(roles))
}

pub async fn add_auto_role(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateAutoRoleDto>,
) -> Result<Json<AutoRoleDto>, ApiError> {
    let role = state.role_panels_uc.add_auto_role(dto.into()).await?;
    Ok(single_dto(role))
}

pub async fn delete_auto_role(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, role_id)): Path<(String, String)>,
) -> Result<Json<()>, ApiError> {
    // Phase 7 B — Gate user : admin+ pour toucher aux auto-roles.
    state
        .role_panels_uc
        .delete_auto_role(&guild_id, &role_id)
        .await?;
    Ok(Json(()))
}
