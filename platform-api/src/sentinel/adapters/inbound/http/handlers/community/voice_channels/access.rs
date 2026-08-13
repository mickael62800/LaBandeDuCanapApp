use super::*;

pub async fn transfer_ownership(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<TransferOwnershipDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let new_owner_name = dto.new_owner_name.clone();

    state
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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<AddCoAdminDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .remove_co_admin(&channel_id, &user_id)
        .await?;

    Ok(ok_response())
}

// ── Whitelist ──

pub async fn get_whitelist(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, owner_id)): Path<(String, String)>,
) -> Result<Json<Vec<WhitelistEntryResponseDto>>, ApiError> {
    let entries = state
        .voice_channels_uc
        .get_whitelist(&guild_id, &owner_id)
        .await?;
    Ok(map_to_dtos(entries))
}

pub async fn add_to_whitelist(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<AddWhitelistDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, owner_id, target_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : moderator+ pour toucher aux permissions voice.
    state
        .voice_channels_uc
        .remove_from_whitelist(&guild_id, &owner_id, &target_id)
        .await?;

    Ok(ok_response())
}

// ── Bans ──

pub async fn ban_from_channel(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
    Json(dto): Json<BanFromChannelDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .voice_channels_uc
        .unban_from_channel(&channel_id, &user_id)
        .await?;

    Ok(ok_response())
}

pub async fn check_ban(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let banned = state
        .voice_channels_uc
        .is_banned(&channel_id, &user_id)
        .await?;
    Ok(Json(serde_json::json!({ "banned": banned })))
}

// ── Invite Links ──
