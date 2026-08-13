use super::*;

pub async fn list_invite_links(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<InviteLinkResponseDto>>, ApiError> {
    let links = state
        .voice_channels_uc
        .list_invite_links(&channel_id)
        .await?;
    Ok(map_to_dtos(links))
}

pub async fn create_invite_link(
    State(state): State<VoiceChannelsState>,
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

    let link = state.voice_channels_uc.create_invite_link(cmd).await?;

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
    State(state): State<VoiceChannelsState>,
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

    let link = state.voice_channels_uc.use_invite_link(cmd).await?;

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
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((channel_id, link_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
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
