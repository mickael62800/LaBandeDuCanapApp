use super::*;

pub async fn list_themes(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<ThemeResponseDto>>, ApiError> {
    let themes = state.voice_channels_uc.list_themes(&guild_id).await?;
    Ok(map_to_dtos(themes))
}

pub async fn create_theme(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Json(dto): Json<CreateThemeDto>,
) -> Result<Json<ThemeResponseDto>, ApiError> {
    // Le theme (channel_name_template...) est consomme par le bot pour nommer
    // les salons -> reserve admin+, scope a la guilde du path (avant : aucune
    // garde -> injection/override cross-serveur).
    let mut cmd: CreateThemeCommand = dto.into();
    cmd.guild_id = guild_id.into();

    let theme = state.voice_channels_uc.create_theme(cmd).await?;
    Ok(single_dto(theme))
}

pub async fn update_theme(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, theme_id)): Path<(String, String)>,
    Json(dto): Json<CreateThemeDto>,
) -> Result<Json<ThemeResponseDto>, ApiError> {
    let mut cmd: CreateThemeCommand = dto.into();
    cmd.guild_id = guild_id.into();

    let theme = state.voice_channels_uc.update_theme(&theme_id, cmd).await?;
    Ok(single_dto(theme))
}

pub async fn delete_theme(
    State(state): State<VoiceChannelsState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, theme_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Phase 7 B — Gate user : admin+ requis pour modifier la config themes voice.
    state
        .voice_channels_uc
        .delete_theme(&guild_id, &theme_id)
        .await?;
    Ok(ok_response())
}
