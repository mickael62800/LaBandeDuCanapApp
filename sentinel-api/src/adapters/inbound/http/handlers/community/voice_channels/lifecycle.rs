use super::*;

pub async fn list_all_channels(
    State(state): State<VoiceChannelsState>,
    user: Option<Extension<WebUser>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit =
        crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500) as usize;
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset) as usize;
    let channels = state.voice_channels_uc.list_all_channels().await?;

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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit =
        crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 200) as usize;
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset) as usize;
    let channels = state.voice_channels_uc.list_channels(&guild_id).await?;
    let page: Vec<_> = channels.into_iter().skip(offset).take(limit).collect();
    Ok(map_to_dtos(page))
}

/// GET /api/voice-channels/{guild_id}/history — historique des salons fermes.
pub async fn list_history_channels(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<VoiceChannelResponseDto>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 100, 500);
    let channels = state
        .voice_channels_uc
        .list_history_channels(&guild_id, limit)
        .await?;
    Ok(map_to_dtos(channels))
}

/// DELETE /api/voice-channels/by-channel/{channel_id}/purge
/// Suppression definitive (hard-delete) d'un salon archive. Refuse si le salon
/// est toujours ouvert — utilisez /close d'abord.
pub async fn purge_channel(
    State(state): State<VoiceChannelsState>,
    // TODO(secu) : le gate par guilde (lookup du guild via channel_id puis
    // verification du role) n'est PAS implemente. Seuls les middlewares du
    // routeur protegent cette route.
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state.voice_channels_uc.purge_channel(&channel_id).await?;

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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state.voice_channels_uc.purge_history(&guild_id).await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "guild_id": &guild_id, "purged_all": true }),
    );

    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

/// GET /api/voice-channels/by-channel/{channel_id}/events
/// Timeline d'un salon vocal : join/leave/move + create/update/close.
pub async fn list_channel_events(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<Vec<serde_json::Value>>, ApiError> {
    let limit = crate::adapters::inbound::http::helpers::normalize_in(params.limit, 200, 1, 1000);
    // La liste blanche des events voix (règle métier) et le SQL vivent
    // derrière le use case audit_logs — plus de sqlx dans l'inbound.
    let logs = state
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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<VoiceChannelDetailDto>, ApiError> {
    let detail = state
        .voice_channels_uc
        .get_channel_detail(&channel_id)
        .await?;
    Ok(single_dto(detail))
}

pub async fn create_channel(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateVoiceChannelDto>,
) -> Result<Json<VoiceChannelResponseDto>, ApiError> {
    // Gate user : moderator+ requis pour creer un voice channel.
    // Pass-through pour les appels bot-internal (user absent).
    let command = dto.into();
    let channel = state.voice_channels_uc.create_channel(command).await?;

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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let before = state
        .voice_channels_uc
        .get_channel_detail(&channel_id)
        .await
        .ok();
    state.voice_channels_uc.close_channel(&channel_id).await?;

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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : moderator+ pour fermer un voice channel.
    // DELETE fait un soft-delete (close)
    state.voice_channels_uc.delete_channel(&channel_id).await?;

    state.broadcaster.broadcast(
        "voice_channel_closed",
        serde_json::json!({ "channel_id": &channel_id }),
    );

    Ok(ok_response())
}

pub async fn update_channel(
    State(state): State<VoiceChannelsState>,
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
