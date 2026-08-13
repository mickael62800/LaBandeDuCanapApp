use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuildUser;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::dto::audit::watched_users::UserDossierResponseDto;
use crate::sentinel::adapters::inbound::http::dto::audit::watched_users::WatchedUserResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::ok_response;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::bootstrap::state::AuditState;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;

#[derive(Debug, Deserialize)]
pub struct WatchedUsersQueryParams {
    pub guild_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_watched_users(
    State(state): State<AuditState>,
    Query(params): Query<WatchedUsersQueryParams>,
) -> Result<Json<Vec<WatchedUserResponseDto>>, ApiError> {
    // IDOR : sans guild_id la liste est GLOBALE (tous serveurs) et les GET
    // echappent au gate global. On exige guild_id + moderator+ scope guilde.
    let guild_id = params.guild_id.clone().ok_or_else(|| {
        ApiError(
            platform_core::sentinel::domain::errors::DomainError::ValidationError(
                "guild_id est obligatoire".into(),
            ),
        )
    })?;
    let limit =
        crate::sentinel::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200);
    let offset = crate::sentinel::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let users = state
        .watched_users_uc
        .list_watched_users(Some(&guild_id), limit, offset)
        .await?;
    Ok(map_to_dtos(users))
}

pub async fn get_user_dossier(
    State(state): State<AuditState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<UserDossierResponseDto>, ApiError> {
    // IDOR + donnees tres sensibles (infractions, notes internes) : le dossier
    // n'etait pas gate alors que add/remove le sont. Reserve moderator+.
    let dossier = state
        .watched_users_uc
        .get_user_dossier(&guild_id, &user_id)
        .await?;
    Ok(single_dto(dossier))
}

#[derive(Debug, Deserialize)]
pub struct AddWatchDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    #[serde(default)]
    pub reason: String,
}

/// POST /api/watched-users — ajouter un utilisateur en surveillance manuelle
pub async fn add_watched_user(
    State(state): State<AuditState>,
    Json(dto): Json<AddWatchDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Meme role que le retrait (remove_watched_user). Guild fourni dans le
    // body -> check_role_for_guild (lookup explicite, bypass superadmin,
    // pass-through si pas de RoleContext = appel bot).
    state
        .watched_users_uc
        .add_manual_watch(&dto.guild_id, &dto.user_id, &dto.username, &dto.reason)
        .await?;

    state.broadcaster.broadcast(
        "watched_user_added",
        serde_json::json!({
            "guild_id": &dto.guild_id,
            "user_id": &dto.user_id,
            "username": &dto.username,
        }),
    );

    Ok(ok_response())
}

/// DELETE /api/watched-users/{guild_id}/{user_id} — retirer de la surveillance manuelle
pub async fn remove_watched_user(
    State(state): State<AuditState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Utilise check_role_for_guild (async, avec bypass superadmin) plutot
    // que check_role (sync, sans bypass).
    state
        .watched_users_uc
        .remove_manual_watch(&guild_id, &user_id)
        .await?;

    Ok(ok_response())
}

#[cfg(test)]
#[path = "tests/watched_users.rs"]
mod tests;
