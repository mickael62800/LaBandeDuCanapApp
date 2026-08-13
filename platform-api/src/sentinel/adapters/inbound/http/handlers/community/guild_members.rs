use crate::sentinel::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use axum::extract::State;
use axum::Extension;
use axum::Json;
use redis::AsyncCommands;
use serde::Deserialize;

use tracing::warn;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::ok_response;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::adapters::outbound::discord_api::DiscordMember;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::domain::entities::community::guild_member::GuildMember;
use platform_core::sentinel::domain::entities::community::guild_member::MemberSummary;
use platform_core::sentinel::domain::entities::community::guild_member_reset::DISCORD_LIST_MEMBERS_CAP;
use platform_core::sentinel::domain::entities::community::guild_member_reset::MEMBERS_CACHE_TTL_SECS;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::ports::inbound::community::manage_members::RegisterMemberCommand;
use platform_core::sentinel::ports::inbound::community::manage_members::SyncMembersCommand;
use platform_core::sentinel::ports::inbound::community::manage_members::UpdateMemberCommand;
/// GET /api/guilds/{guild_id}/members — liste les membres Discord (cache 10min, fallback Discord API)
pub async fn list_members(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<DiscordMember>>, ApiError> {
    let cache_key = format!("guild:members:{guild_id}");

    // Cache-first
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(Some(json)) = conn.get::<_, Option<String>>(&cache_key).await {
            if let Ok(members) = serde_json::from_str::<Vec<DiscordMember>>(&json) {
                return Ok(Json(members));
            }
        }
    }

    let members = state
        .discord_api
        .list_members(&guild_id, DISCORD_LIST_MEMBERS_CAP)
        .await?;

    // Populate cache
    if let Ok(mut conn) = state.redis_client.get_multiplexed_async_connection().await {
        if let Ok(json) = serde_json::to_string(&members) {
            if let Err(e) = conn
                .set_ex::<_, _, ()>(&cache_key, json, MEMBERS_CACHE_TTL_SECS)
                .await
            {
                warn!(error = %e, cache_key = %cache_key, "Echec cache set members");
            }
        }
    }

    Ok(Json(members))
}

/// GET /api/members/{guild_id} — liste les membres depuis la BDD
pub async fn list_members_db(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<GuildMember>>, ApiError> {
    let members = state.members_uc.list_members(&guild_id).await?;
    Ok(Json(members))
}

/// GET /api/members/{guild_id}/{user_id} — profil d'un membre
pub async fn get_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<GuildMember>, ApiError> {
    let member = state.members_uc.get_member(&guild_id, &user_id).await?;
    Ok(Json(member))
}

/// GET /api/members/{guild_id}/{user_id}/summary — profil complet agrege
pub async fn get_member_summary(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<MemberSummary>, ApiError> {
    let summary = state
        .members_uc
        .get_member_summary(&guild_id, &user_id)
        .await?;
    Ok(Json(summary))
}

/// POST /api/members/sync — sync bulk depuis un bot
pub async fn sync_members(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(payload): Json<SyncMembersPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .members_uc
        .sync_members(SyncMembersCommand {
            guild_id: payload.guild_id,
            members: payload.members,
        })
        .await?;
    Ok(Json(serde_json::json!({ "synced": count })))
}

/// POST /api/members/register — enregistre un nouveau membre
pub async fn register_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(member): Json<GuildMember>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .members_uc
        .register_member(RegisterMemberCommand { member })
        .await?;
    Ok(ok_response())
}

/// DELETE /api/members/{guild_id}/{user_id} — supprime un membre
pub async fn remove_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : moderator+ requis pour retirer un membre du cache local.
    state.members_uc.remove_member(&guild_id, &user_id).await?;
    Ok(ok_response())
}

/// PATCH /api/members/{guild_id}/{user_id} — met a jour un membre
pub async fn update_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
    Json(payload): Json<UpdateMemberPayload>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Gate user : admin+ requis (ce handler ecrit aussi le champ `roles`).
    state
        .members_uc
        .update_member(UpdateMemberCommand {
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: payload.username,
            display_name: payload.display_name,
            avatar: payload.avatar,
            roles: payload.roles,
        })
        .await?;
    Ok(ok_response())
}

#[derive(Deserialize)]
pub struct SyncMembersPayload {
    pub guild_id: GuildId,
    pub members: Vec<GuildMember>,
}

/// POST /api/members/{guild_id}/{user_id}/reset — nettoie TOUTES les donnees
/// de moderation d'un membre sur une guild en une seule transaction.
///
/// Supprime :
/// - infractions (table `infractions`)
/// Les traces d'actions dans `audit_logs` sont conservees selon la politique
/// de retention et ne font pas partie de cette remise a zero.
/// - strikes (`user_strikes`)
/// - notes moderateurs (`user_notes`)
/// - surveillance manuelle (`manual_watched_users`)
/// - rappels de sanction (`sanction_reminders`, par target_id)
///
/// **Operation irreversible**, gatee derriere `Role::Admin` + bypass superadmin.
/// Tout se fait dans une transaction atomique : en cas d'erreur sur un DELETE,
/// on rollback et on retourne l'erreur — l'etat DB reste coherent.
pub async fn reset_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let totals: serde_json::Map<String, serde_json::Value> = state
        .members_uc
        .reset_member(&guild_id, &user_id)
        .await?
        .into_iter()
        .map(|(key, rows)| (key.to_string(), rows.into()))
        .collect();

    tracing::info!(
        guild_id = %guild_id,
        user_id = %user_id,
        "reset_member effectue"
    );

    state.broadcaster.broadcast(
        "member_reset",
        serde_json::json!({
            "guild_id": &guild_id,
            "user_id": &user_id,
            "totals": &totals,
        }),
    );

    Ok(Json(serde_json::json!({
        "status": "ok",
        "guild_id": guild_id,
        "user_id": user_id,
        "totals": totals,
    })))
}

#[derive(Deserialize)]
pub struct UpdateMemberPayload {
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub roles: Option<serde_json::Value>,
}

/// POST /api/members/{guild_id}/{user_id}/leave
///
/// Marque un membre comme parti :
/// - guild_members.left_at = NOW() (idempotent : ne reset pas si deja parti)
///
/// Les autres donnees (infractions, audit_logs, stats, tickets)
/// sont conservees pour la chaine de moderation et l'historique.
///
/// Endpoint appele par sentinel-bot sur GuildMemberRemove. Idempotent.
pub async fn leave_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows_affected = state.members_uc.leave_member(&guild_id, &user_id).await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "rows_affected": rows_affected,
    })))
}

/// POST /api/members/{guild_id}/{user_id}/rejoin
///
/// Marque un membre comme revenu :
/// - guild_members.left_at = NULL
/// - guild_members.joined_at = NOW()
///
/// Le wallet reste a 0 (le user repart de zero cote jeu).
/// Les donnees non-jeu (infractions, etc.) sont automatiquement re-attachees
/// via l'ID Discord stable, pas besoin de re-importer.
///
/// Endpoint appele par sentinel-bot sur GuildMemberAdd. Idempotent.
pub async fn rejoin_member(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<serde_json::Value>, ApiError> {
    let rows_affected = state.members_uc.rejoin_member(&guild_id, &user_id).await?;

    Ok(Json(serde_json::json!({
        "ok": true,
        "rows_affected": rows_affected,
    })))
}
