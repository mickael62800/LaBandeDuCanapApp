use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::community::voice_channels::AddCoAdminDto;
use crate::adapters::inbound::http::dto::community::voice_channels::AddWhitelistDto;
use crate::adapters::inbound::http::dto::community::voice_channels::BanFromChannelDto;
use crate::adapters::inbound::http::dto::community::voice_channels::CreateInviteLinkDto;
use crate::adapters::inbound::http::dto::community::voice_channels::CreateThemeDto;
use crate::adapters::inbound::http::dto::community::voice_channels::CreateVoiceChannelDto;
use crate::adapters::inbound::http::dto::community::voice_channels::InviteLinkResponseDto;
use crate::adapters::inbound::http::dto::community::voice_channels::ThemeResponseDto;
use crate::adapters::inbound::http::dto::community::voice_channels::TransferOwnershipDto;
use crate::adapters::inbound::http::dto::community::voice_channels::UpdateVoiceChannelDto;
use crate::adapters::inbound::http::dto::community::voice_channels::UseInviteLinkDto;
use crate::adapters::inbound::http::dto::community::voice_channels::VoiceChannelDetailDto;
use crate::adapters::inbound::http::dto::community::voice_channels::VoiceChannelResponseDto;
use crate::adapters::inbound::http::dto::community::voice_channels::WhitelistEntryResponseDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::adapters::inbound::http::state::AppState;
use sentinel_core::domain::errors::DomainError;

/// Ensemble des guilds ou le caller est Moderator+ (pour scoper les endpoints
/// guild-less comme `list_all_channels`). Délègue au use case tickets (source
/// unique de la règle, plus de SQL dupliqué dans l'inbound).
async fn moderated_guilds(
    state: &AppState,
    user_id: &str,
) -> Result<std::collections::HashSet<String>, ApiError> {
    Ok(state.system.tickets_uc.moderated_guilds(user_id).await?)
}
use sentinel_core::domain::entities::system::discord_ids::ChannelId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateThemeCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;

async fn log_voice_event(
    state: &AppState,
    guild_id: GuildId,
    event_type: &str,
    channel_id: ChannelId,
    channel_name: Option<String>,
    actor_id: Option<String>,
    actor_name: Option<String>,
    details: serde_json::Value,
) {
    let cmd = CreateAuditLogCommand {
        guild_id,
        event_type: event_type.to_string(),
        actor_id,
        actor_name,
        target_id: None,
        target_name: None,
        channel_id: Some(channel_id.into()),
        channel_name,
        details,
    };
    if let Err(e) = state.audit.audit_logs_uc.create(cmd).await {
        tracing::warn!("failed to log voice audit event: {e}");
    }
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Channels ──

pub async fn list_all_channels(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit =
        crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500) as usize;
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset) as usize;
    let channels = state
        .community
        .voice_channels_uc
        .list_all_channels()
        .await?;

    // Endpoint guild-less : on scope au web. Le chemin bot/interne (pas de
    // WebUser) n'est PAS filtre. Un superadmin voit tout ; sinon on ne
    // retourne que les salons des guilds ou le caller est Moderator+ (mirroir
    // de `list_tickets`).
    let channels = match user.as_ref() {
        None => channels,
        Some(Extension(ctx)) => {
            if state
                .superadmin_user_ids
                .iter()
                .any(|sid| sid == &ctx.discord_user_id)
            {
                channels
            } else {
                let allowed = moderated_guilds(&state, &ctx.discord_user_id).await?;
                channels
                    .into_iter()
                    .filter(|c| allowed.contains(c.guild_id.as_str()))
                    .collect()
            }
        }
    };

    let page: Vec<_> = channels.into_iter().skip(offset).take(limit).collect();
    Ok(map_to_dtos(page))
}

pub async fn list_channels(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit =
        crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200) as usize;
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset) as usize;
    let channels = state
        .community
        .voice_channels_uc
        .list_channels(&guild_id)
        .await?;
    let page: Vec<_> = channels.into_iter().skip(offset).take(limit).collect();
    Ok(map_to_dtos(page))
}

/// GET /api/voice-channels/{guild_id}/history — historique des salons fermes.
pub async fn list_history_channels(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 100, 500);
    let channels = state
        .community
        .voice_channels_uc
        .list_history_channels(&guild_id, limit)
        .await?;
    Ok(map_to_dtos(channels))
}

/// DELETE /api/voice-channels/by-channel/{channel_id}/purge
/// Suppression definitive (hard-delete) d'un salon archive. Refuse si le salon
/// est toujours ouvert — utilisez /close d'abord.
pub async fn purge_channel(
    State(state): State<AppState>,
    // TODO(secu) : le gate par guilde (lookup du guild via channel_id puis
    // verification du role) n'est PAS implemente. Seuls les middlewares du
    // routeur protegent cette route.
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state
        .community
        .voice_channels_uc
        .purge_channel(&channel_id)
        .await?;

    if !deleted {
        return Err(ApiError(DomainError::ValidationError(
            "salon introuvable ou encore ouvert (fermez-le d'abord)".into(),
        )));
    }

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "channel_id": &channel_id, "purged": true }),
    );

    Ok(ok_response())
}

/// DELETE /api/voice-channels/{guild_id}/history
/// Purge (hard-delete) tous les salons fermes d'une guild.
pub async fn purge_history(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state
        .community
        .voice_channels_uc
        .purge_history(&guild_id)
        .await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "guild_id": &guild_id, "purged_all": true }),
    );

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

/// GET /api/voice-channels/by-channel/{channel_id}/events
/// Timeline d'un salon vocal : join/leave/move + create/update/close.
pub async fn list_channel_events(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_in(params.limit, 200, 1, 1000);
    // La liste blanche des events voix (règle métier) et le SQL vivent
    // derrière le use case audit_logs — plus de sqlx dans l'inbound.
    let logs = state
        .audit
        .audit_logs_uc
        .list_voice_channel_events(&channel_id, limit)
        .await?;

    let events: Vec<serde_json::Value> = logs
        .into_iter()
        .map(|l| {
            serde_json::json!({
                "id": l.id.to_string(),
                "guild_id": l.guild_id,
                "event_type": l.event_type,
                "actor_id": l.actor_id,
                "actor_name": l.actor_name,
                "channel_id": l.channel_id,
                "channel_name": l.channel_name,
                "details": l.details,
                "created_at": l.created_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(events))
}

pub async fn get_channel_detail(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<VoiceChannelDetailDto>, ApiError> {
    let detail = state
        .community
        .voice_channels_uc
        .get_channel_detail(&channel_id)
        .await?;
    Ok(single_dto(detail))
}

pub async fn create_channel(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateVoiceChannelDto>,
) -> Result<Json<VoiceChannelResponseDto>, ApiError> {
    // Gate user : moderator+ requis pour creer un voice channel.
    // Pass-through pour les appels bot-internal (user absent).
    let command = dto.into();
    let channel = state
        .community
        .voice_channels_uc
        .create_channel(command)
        .await?;

    log_voice_event(
        &state,
        channel.guild_id.clone(),
        "voice_channel_created",
        channel.channel_id.clone(),
        Some(channel.channel_name.clone()),
        Some(channel.owner_id.clone()),
        Some(channel.owner_name.clone()),
        serde_json::json!({
            "kind": channel.kind.as_str(),
            "visibility": channel.visibility,
            "queue_enabled": channel.queue_enabled,
            "stage_enabled": channel.stage_enabled,
            "member_limit": channel.member_limit,
        }),
    )
    .await;

    state.broadcaster.broadcast(
        "voice_channel_created",
        serde_json::json!({
            "guild_id": &channel.guild_id,
            "id": channel.id.to_string(),
            "channel_name": &channel.channel_name,
            "owner_name": &channel.owner_name,
            "kind": channel.kind.as_str(),
        }),
    );

    Ok(single_dto(channel))
}

pub async fn close_channel(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let before = state
        .community
        .voice_channels_uc
        .get_channel_detail(&channel_id)
        .await
        .ok();
    state
        .community
        .voice_channels_uc
        .close_channel(&channel_id)
        .await?;

    let payload = if let Some(d) = &before {
        log_voice_event(
            &state,
            d.channel.guild_id.clone(),
            "voice_channel_closed",
            channel_id.clone().into(),
            Some(d.channel.channel_name.clone()),
            None,
            None,
            serde_json::json!({}),
        )
        .await;
        serde_json::json!({
            "id": d.channel.id,
            "channel_id": &channel_id,
            "guild_id": &d.channel.guild_id,
            "actor": { "source": "web" },
        })
    } else {
        serde_json::json!({ "channel_id": &channel_id, "actor": { "source": "web" } })
    };
    state.broadcaster.broadcast("voice_channel_closed", payload);

    Ok(ok_response())
}

pub async fn delete_channel(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : moderator+ pour fermer un voice channel.
    // DELETE fait un soft-delete (close)
    state
        .community
        .voice_channels_uc
        .delete_channel(&channel_id)
        .await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(ok_response())
}

pub async fn update_channel(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<UpdateVoiceChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let changes = serde_json::json!({
        "visibility": dto.visibility.clone(),
        "locked": dto.locked,
        "queue_enabled": dto.queue_enabled,
        "name": dto.name.clone(),
        "status": dto.status.clone(),
        "member_limit": dto.member_limit,
        "stage_enabled": dto.stage_enabled,
    });

    state
        .community
        .voice_channels_uc
        .update_channel(UpdateVoiceChannelCommand {
            channel_id: channel_id.clone().into(),
            visibility: dto.visibility,
            locked: dto.locked,
            queue_enabled: dto.queue_enabled,
            name: dto.name,
            status: dto.status,
            member_limit: dto.member_limit,
            queue_channel_id: dto.queue_channel_id,
            stage_enabled: dto.stage_enabled,
        })
        .await?;

    let detail_opt = state
        .community
        .voice_channels_uc
        .get_channel_detail(&channel_id)
        .await
        .ok();
    if let Some(detail) = &detail_opt {
        log_voice_event(
            &state,
            detail.channel.guild_id.clone(),
            "voice_channel_updated",
            channel_id.clone().into(),
            Some(detail.channel.channel_name.clone()),
            None,
            None,
            changes,
        )
        .await;
    }

    // Sync bilateral : enrichi avec id (UUID DB), etat complet, et
    // actor.source = "web" pour que le bot listener re-render le panel.
    let payload = if let Some(detail) = &detail_opt {
        serde_json::json!({
            "id": detail.channel.id,
            "channel_id": &channel_id,
            "guild_id": &detail.channel.guild_id,
            "owner_id": &detail.channel.owner_id,
            "visibility": &detail.channel.visibility,
            "locked": detail.channel.locked,
            "queue_enabled": detail.channel.queue_enabled,
            "actor": { "source": "web" },
        })
    } else {
        serde_json::json!({ "channel_id": &channel_id, "actor": { "source": "web" } })
    };
    state
        .broadcaster
        .broadcast("voice_channel_updated", payload);

    Ok(ok_response())
}

pub async fn transfer_ownership(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<TransferOwnershipDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let new_owner_name = dto.new_owner_name.clone();

    state
        .community
        .voice_channels_uc
        .transfer_ownership(TransferOwnershipCommand {
            channel_id: channel_id.clone().into(),
            new_owner_id: dto.new_owner_id,
            new_owner_name: dto.new_owner_name,
        })
        .await?;

    state.broadcaster.broadcast(
        "voice_channel_updated",
        serde_json::json!({
            "channel_id": &channel_id,
            "event": "transfer",
            "new_owner": &new_owner_name,
        }),
    );

    Ok(ok_response())
}

// ── Co-admins ──

pub async fn add_co_admin(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<AddCoAdminDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .community
        .voice_channels_uc
        .add_co_admin(ManageCoAdminCommand {
            channel_id: channel_id.into(),
            user_id: dto.user_id,
            user_name: dto.user_name,
        })
        .await?;

    Ok(ok_response())
}

pub async fn remove_co_admin(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .community
        .voice_channels_uc
        .remove_co_admin(&channel_id, &user_id)
        .await?;

    Ok(ok_response())
}

// ── Whitelist ──

pub async fn get_whitelist(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, owner_id)): Path<(String, String)>,
) -> Result<Json<Vec<WhitelistEntryResponseDto>>, ApiError> {
    let entries = state
        .community
        .voice_channels_uc
        .get_whitelist(&guild_id, &owner_id)
        .await?;
    Ok(map_to_dtos(entries))
}

pub async fn add_to_whitelist(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<AddWhitelistDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .community
        .voice_channels_uc
        .add_to_whitelist(ManageWhitelistCommand {
            guild_id: dto.guild_id,
            owner_id: dto.owner_id,
            target_id: dto.target_id,
            target_name: dto.target_name,
        })
        .await?;

    Ok(ok_response())
}

pub async fn remove_from_whitelist(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, owner_id, target_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : moderator+ pour toucher aux permissions voice.
    state
        .community
        .voice_channels_uc
        .remove_from_whitelist(&guild_id, &owner_id, &target_id)
        .await?;

    Ok(ok_response())
}

// ── Bans ──

pub async fn ban_from_channel(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<BanFromChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .community
        .voice_channels_uc
        .ban_from_channel(BanFromChannelCommand {
            channel_id: channel_id.into(),
            user_id: dto.user_id,
            user_name: dto.user_name,
            banned_by: dto.banned_by,
            reason: dto.reason,
            duration_secs: dto.duration_secs,
        })
        .await?;

    Ok(ok_response())
}

pub async fn unban_from_channel(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .community
        .voice_channels_uc
        .unban_from_channel(&channel_id, &user_id)
        .await?;

    Ok(ok_response())
}

pub async fn check_ban(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let banned = state
        .community
        .voice_channels_uc
        .is_banned(&channel_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "banned": banned })))
}

// ── Invite Links ──

pub async fn list_invite_links(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<InviteLinkResponseDto>>, ApiError> {
    let links = state
        .community
        .voice_channels_uc
        .list_invite_links(&channel_id)
        .await?;
    Ok(map_to_dtos(links))
}

pub async fn create_invite_link(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<CreateInviteLinkDto>,
) -> Result<Json<InviteLinkResponseDto>, ApiError> {
    let cmd = CreateInviteLinkCommand {
        channel_id: channel_id.clone().into(),
        created_by: dto.created_by,
        created_by_name: dto.created_by_name,
        duration_secs: dto.duration_secs,
        max_uses: dto.max_uses,
    };

    let link = state
        .community
        .voice_channels_uc
        .create_invite_link(cmd)
        .await?;

    state.broadcaster.broadcast(
        "voice_invite_created",
        serde_json::json!({
            "channel_id": &channel_id,
            "code": &link.code,
            "created_by_name": &link.created_by_name,
        }),
    );

    Ok(single_dto(link))
}

pub async fn use_invite_link(
    State(state): State<AppState>,
    user: Option<Extension<WebUser>>,
    Path(code): Path<String>,
    Json(dto): Json<UseInviteLinkDto>,
) -> Result<Json<InviteLinkResponseDto>, ApiError> {
    // Action self-service : un utilisateur consomme SON propre code.
    // Pour un appel WEB (WebUser present), l'identite a whitelister est
    // DERIVEE du principal authentifie -> on IGNORE tout `user_id` fourni dans
    // le body (anti-forgery : un caller ne peut whitelister que lui-meme).
    // Pour le chemin bot/interne (WebUser absent, gRPC/Bearer de confiance),
    // on conserve le `user_id` du body (le bot passe le vrai redeemer).
    let user_id = match user.as_ref() {
        Some(Extension(ctx)) => ctx.discord_user_id.clone().into(),
        None => dto.user_id.clone(),
    };

    let cmd = UseInviteLinkCommand {
        code: code.clone(),
        user_id: user_id.clone(),
        user_name: dto.user_name,
    };

    let link = state
        .community
        .voice_channels_uc
        .use_invite_link(cmd)
        .await?;

    state.broadcaster.broadcast(
        "voice_invite_used",
        serde_json::json!({
            "channel_id": &link.channel_id,
            "code": &code,
            "user_id": &user_id,
        }),
    );

    Ok(single_dto(link))
}

pub async fn revoke_invite_link(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, link_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .community
        .voice_channels_uc
        .revoke_invite_link(&channel_id, &link_id)
        .await?;

    state.broadcaster.broadcast(
        "voice_invite_revoked",
        serde_json::json!({ "channel_id": &channel_id, "link_id": &link_id }),
    );

    Ok(ok_response())
}

// ── Themes ──

pub async fn list_themes(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<ThemeResponseDto>>, ApiError> {
    let themes = state
        .community
        .voice_channels_uc
        .list_themes(&guild_id)
        .await?;
    Ok(map_to_dtos(themes))
}

pub async fn create_theme(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateThemeDto>,
) -> Result<Json<ThemeResponseDto>, ApiError> {
    // Le theme (channel_name_template...) est consomme par le bot pour nommer
    // les salons -> reserve admin+, scope a la guilde du path (avant : aucune
    // garde -> injection/override cross-serveur).
    let mut cmd: CreateThemeCommand = dto.into();
    cmd.guild_id = guild_id.into();

    let theme = state.community.voice_channels_uc.create_theme(cmd).await?;
    Ok(single_dto(theme))
}

pub async fn update_theme(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, theme_id)): Path<(String, String)>,
    Json(dto): Json<CreateThemeDto>,
) -> Result<Json<ThemeResponseDto>, ApiError> {
    let mut cmd: CreateThemeCommand = dto.into();
    cmd.guild_id = guild_id.into();

    let theme = state
        .community
        .voice_channels_uc
        .update_theme(&theme_id, cmd)
        .await?;
    Ok(single_dto(theme))
}

pub async fn delete_theme(
    State(state): State<AppState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, theme_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : admin+ requis pour modifier la config themes voice.
    state
        .community
        .voice_channels_uc
        .delete_theme(&guild_id, &theme_id)
        .await?;
    Ok(ok_response())
}

#[cfg(test)]
#[path = "tests/voice_channels.rs"]
mod tests;
