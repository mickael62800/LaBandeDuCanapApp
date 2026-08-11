use super::*;

pub async fn log_action(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Json(mut dto): Json<LogActionDto>,
) -> Result<Json<ModerationActionResponseDto>, ApiError> {
    // S1/S4 — Liaison de l'identite moderateur au principal authentifie.
    // Pour un appelant WEB (WebUser present via token Discord) on derive
    // `moderator_id`/`moderator_name` de l'identite verifiee et on IGNORE les
    // valeurs du body (anti-usurpation). Pour le bot/interne (gRPC/Bearer, pas
    // de WebUser) on conserve les valeurs du body : le bot transmet le vrai
    // moderateur. NB : le bot passe par gRPC, ce handler HTTP est web-only.
    if let Some(Extension(ctx)) = &user {
        dto.moderator_id = ctx.discord_user_id.clone();
        dto.moderator_name = ctx.discord_user_id.clone();
    }

    // Validation
    validation::validate_moderation_action(
        &dto.guild_id,
        &dto.moderator_id,
        &dto.target_id,
        &dto.reason,
        &dto.action_type,
    )
    .map_err(ApiError)?;

    // Phase 7B — Gate user (pass-through pour les appels bot/internal sans token Discord).

    let action_type = dto.action_type.clone();
    let target_name = dto.target_name.clone();
    let moderator_name = dto.moderator_name.clone();
    let reason = dto.reason.clone();

    let guild_id = dto.guild_id.clone();
    let target_id = dto.target_id.clone();
    let _moderator_id = dto.moderator_id.clone();
    let _duration = dto.duration;

    let command = dto.into();
    // Orchestration atomique (action + strike) dans le service.
    let logged = state.moderation_uc.log_action_with_strike(command).await?;
    let action = logged.action;
    let strike_result = logged.strike;

    let mut dto = ModerationActionResponseDto::from(action);
    if let Some(ref sr) = strike_result {
        dto.strikes_count = Some(sr.active_count);
        dto.escalation_action = sr.escalation_action.clone();
        dto.escalation_duration = sr.escalation_duration;
    }

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": action_type,
            "target_id": target_id,
            "target_name": target_name,
            "moderator_name": moderator_name,
            "reason": reason,
            "guild_id": guild_id,
        }),
    );

    if let Some(ref sr) = strike_result {
        if sr.should_trigger_escalation_broadcast() {
            state.broadcaster.broadcast(
                "strike_added",
                serde_json::json!({
                    "guild_id": guild_id,
                    "user_id": target_id,
                    "active_count": sr.active_count,
                    "escalation_action": sr.escalation_action,
                    "escalation_duration": sr.escalation_duration,
                }),
            );
        }
    }

    // Auto-create reminder for temporary sanctions (regle metier : voir
    // `ModerationActionType::is_temporary` dans domain/value_objects).

    Ok(Json(dto))
}

#[derive(Debug, Deserialize)]
pub struct ExecuteBanDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    /// Phase 1 sync (cf. SYNC_DISCORD_WEB_DESIGN.md) : si fourni, l API
    /// publie un event `moderation.ban.executed` avec cet `action_id`,
    /// permettant au bot d editer le message Discord correspondant.
    #[serde(default)]
    pub action_id: Option<uuid::Uuid>,
}

/// POST /api/moderation/execute-ban — execute un ban Discord + log l'action
pub async fn execute_ban(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<ExecuteBanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_reason(&dto.reason).map_err(ApiError)?;

    state
        .discord_api
        .ban_user(&dto.guild_id, &dto.user_id, &dto.reason)
        .await
        .map_err(ApiError)?;

    let reason = dto.reason.clone();

    // S1/S4 — identite moderateur : derivee du principal authentifie pour le
    // web (WebUser), sinon valeurs desktop par defaut (appel interne).
    let (moderator_id, moderator_name) = resolve_web_moderator(&user, "desktop", "Desktop App");

    let command =
        sentinel_core::ports::inbound::moderation::manage_moderation::LogModerationCommand {
            guild_id: dto.guild_id.clone(),
            channel_id: String::new().into(),
            moderator_id: moderator_id.clone(),
            moderator_name: moderator_name.clone(),
            target_id: dto.user_id.clone().into(),
            target_name: dto.user_id.clone().into(),
            action_type: "ban_permanent".into(),
            reason: dto.reason,
            gravity: None,
            duration: None,
        };
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "ban_permanent",
            "target_id": &dto.user_id,
            "target_name": &dto.user_id,
            "moderator_name": &moderator_name,
            "guild_id": &dto.guild_id,
            "reason": &reason,
        }),
    );

    // Phase 1 sync : event dedie pour le bot et le web (refresh + edit
    // message Discord). Format aligne sur SYNC_DISCORD_WEB_DESIGN.md.
    if let Some(action_id) = dto.action_id {
        state.broadcaster.broadcast(
            "moderation.ban.executed",
            serde_json::json!({
                "action_id": action_id,
                "guild_id": &dto.guild_id,
                "target_id": &dto.user_id,
                "actor": { "user_id": &moderator_id, "source": "web" },
                "reason": &reason,
            }),
        );
    }

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct ExecuteMuteDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    /// Duree du timeout en secondes. Defaut : 1h. Max : 28 jours (clamp cote Discord).
    #[serde(default)]
    pub duration: Option<u64>,
    /// Nom d'affichage optionnel (stocke dans audit_logs.target_name).
    #[serde(default)]
    pub target_name: Option<String>,
}

/// POST /api/moderation/execute-mute — applique un timeout Discord + log l'action
pub async fn execute_mute(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<ExecuteMuteDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("user_id", &dto.user_id).map_err(ApiError)?;
    validation::validate_reason(&dto.reason).map_err(ApiError)?;

    let duration =
        sentinel_core::domain::entities::moderation::review::manual::resolve_mute_duration(
            dto.duration,
        );
    state
        .discord_api
        .apply_timeout(&dto.guild_id, &dto.user_id, duration)
        .await
        .map_err(ApiError)?;

    let target_name = dto
        .target_name
        .unwrap_or_else(|| dto.user_id.clone().into());

    // S1/S4 — identite moderateur derivee du principal authentifie (web).
    let (moderator_id, moderator_name) = resolve_web_moderator(&user, "web-panel", "Web Admin");

    let command =
        sentinel_core::ports::inbound::moderation::manage_moderation::LogModerationCommand {
            guild_id: dto.guild_id.clone(),
            channel_id: String::new().into(),
            moderator_id,
            moderator_name: moderator_name.clone(),
            target_id: dto.user_id.clone().into(),
            target_name: target_name.clone(),
            action_type: "mute".into(),
            reason: dto.reason.clone(),
            gravity: None,
            duration: Some(duration),
        };
    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "mute",
            "target_id": &dto.user_id,
            "target_name": &target_name,
            "moderator_name": &moderator_name,
            "guild_id": &dto.guild_id,
            "reason": &dto.reason,
            "duration": duration,
        }),
    );

    Ok(ok_response())
}

#[derive(Debug, Deserialize)]
pub struct ExecuteUnbanDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
}

/// POST /api/moderation/execute-unban — debannir un utilisateur Discord
pub async fn execute_unban(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<ExecuteUnbanDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Validation
    validation::validate_guild_user_path(&dto.guild_id, &dto.user_id).map_err(ApiError)?;

    state
        .discord_api
        .unban_user(&dto.guild_id, &dto.user_id)
        .await
        .map_err(ApiError)?;

    let target_id = dto.user_id.clone();
    let guild_id = dto.guild_id.clone();

    // S1/S4 — identite moderateur derivee du principal authentifie (web).
    let (moderator_id, moderator_name) = resolve_web_moderator(&user, "desktop", "Desktop App");

    let command =
        sentinel_core::ports::inbound::moderation::manage_moderation::LogModerationCommand {
            guild_id: dto.guild_id,
            channel_id: String::new().into(),
            moderator_id,
            moderator_name: moderator_name.clone(),
            target_id: target_id.clone().into(),
            target_name: target_id.clone().into(),
            action_type: "unban".into(),
            reason: "Deban depuis le desktop".into(),
            gravity: None,
            duration: None,
        };
    state
        .moderation_uc
        .delete_bans_for_user(&guild_id, &target_id)
        .await?;

    state.moderation_uc.log_action(command).await?;

    state.broadcaster.broadcast(
        "moderation_action",
        serde_json::json!({
            "action_type": "unban",
            "target_id": &target_id,
            "moderator_name": &moderator_name,
            "guild_id": &guild_id,
        }),
    );

    Ok(ok_response())
}
