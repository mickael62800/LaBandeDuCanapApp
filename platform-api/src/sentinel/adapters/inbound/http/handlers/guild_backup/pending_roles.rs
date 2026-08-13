//! Handlers HTTP des re-attributions de roles en attente (`guild_backup`).
//!
//! Meme gate RBAC que le reste du domaine : **Owner** requis (le bot, appels
//! internes sans `X-Discord-Token`, contourne via `check_role_for_guild`).
//!
//! Flux : au restore le bot POST la liste des grants ({user_id, role_ids}) ;
//! au join d'un membre il POST `/consume` qui renvoie ET supprime ses roles
//! (atomique) ; `DELETE` purge la guild (nouveau restore propre).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use crate::sentinel::bootstrap::state::GuildBackupState;
use platform_core::sentinel::domain::entities::guild_backup::pending_role_grant::PendingRoleGrant;

/// Une entree de re-attribution recue au restore (body de `save`).
#[derive(Debug, Deserialize)]
pub struct PendingRoleGrantDto {
    pub user_id: String,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SavedGrantsDto {
    /// Nombre d'entrees ecrites (grants vides ignores).
    pub saved: u64,
}

#[derive(Debug, Serialize)]
pub struct ConsumedGrantDto {
    /// Roles a re-attribuer au membre (vide si aucune entree en attente).
    pub role_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClearedGrantsDto {
    pub cleared: u64,
}

/// POST /api/guild-backup/{guild_id}/pending-roles — enregistre les grants.
/// Body = liste de `{user_id, role_ids}`. Owner requis (bypass interne bot).
pub async fn save_pending_roles(
    State(state): State<GuildBackupState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(body): Json<Vec<PendingRoleGrantDto>>,
) -> Result<(StatusCode, Json<SavedGrantsDto>), ApiError> {
    let grants: Vec<PendingRoleGrant> = body
        .into_iter()
        .map(|g| PendingRoleGrant {
            guild_id: guild_id.clone(),
            user_id: g.user_id,
            role_ids: g.role_ids,
        })
        .collect();
    let saved = state
        .pending_role_grants_uc
        .save_grants(&guild_id, grants)
        .await?;
    Ok((StatusCode::OK, Json(SavedGrantsDto { saved })))
}

/// POST /api/guild-backup/{guild_id}/pending-roles/{user_id}/consume — lit ET
/// supprime (atomique) les roles en attente d'un membre. `role_ids` vide si
/// aucune entree.
pub async fn consume_pending_roles(
    State(state): State<GuildBackupState>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<ConsumedGrantDto>, ApiError> {
    let role_ids = state
        .pending_role_grants_uc
        .take_grant(&guild_id, &user_id)
        .await?
        .unwrap_or_default();
    Ok(Json(ConsumedGrantDto { role_ids }))
}

/// DELETE /api/guild-backup/{guild_id}/pending-roles — purge la guild.
pub async fn clear_pending_roles(
    State(state): State<GuildBackupState>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<ClearedGrantsDto>, ApiError> {
    let cleared = state.pending_role_grants_uc.clear_guild(&guild_id).await?;
    Ok(Json(ClearedGrantsDto { cleared }))
}
