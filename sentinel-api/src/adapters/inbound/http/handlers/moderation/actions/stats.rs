use super::*;

/// MOD #7 — GET /api/moderation/modstats/{guild_id}
///
/// Agrege les actions de moderation par moderateur sur les 30 derniers jours.
/// Retourne le top 20 classe par nombre total d'actions decroissant.
///
/// Lecture deleguee au use case `modstats_uc` (read-only, aggregation simple).
pub async fn get_modstats(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<TrendQuery>,
) -> Result<
    Json<Vec<crate::adapters::inbound::http::dto::moderation::actions::ModStatsEntryDto>>,
    ApiError,
> {
    let days =
        (crate::adapters::inbound::http::helpers::normalize_in(params.days, 30, 1, 90)) as i32;

    let rows = state.modstats_uc.modstats(&guild_id, days).await?;

    let dtos = rows
        .into_iter()
        .map(
            |r| crate::adapters::inbound::http::dto::moderation::actions::ModStatsEntryDto {
                moderator_id: r.moderator_id,
                moderator_name: r.moderator_name,
                total: r.total,
                warns: r.warns,
                mutes: r.mutes,
                bans: r.bans,
                kicks: r.kicks,
            },
        )
        .collect();

    Ok(Json(dtos))
}

/// GET /api/moderation/modstats/{guild_id}/trend?days=30
///
/// Retourne les actions de moderation agregees par jour sur les N derniers
/// jours (default 30, max 90). Lecture depuis `audit_logs` comme modstats.
/// Utilise pour la courbe "Tendance moderation" sur la page web /modstats.
pub async fn get_modstats_trend(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<TrendQuery>,
) -> Result<Json<Vec<ModstatsTrendDayDto>>, ApiError> {
    let days =
        (crate::adapters::inbound::http::helpers::normalize_in(params.days, 30, 1, 90)) as i32;

    let rows = state.modstats_uc.modstats_trend(&guild_id, days).await?;

    let dtos = rows
        .into_iter()
        .map(|r| ModstatsTrendDayDto {
            day: r.day.to_string(),
            warns: r.warns,
            mutes: r.mutes,
            bans: r.bans,
            kicks: r.kicks,
        })
        .collect();

    Ok(Json(dtos))
}

#[derive(serde::Deserialize)]
pub struct TrendQuery {
    pub days: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct ModstatsTrendDayDto {
    pub day: String,
    pub warns: i64,
    pub mutes: i64,
    pub bans: i64,
    pub kicks: i64,
}

/// DELETE /api/moderation/actions/{id} — annule une action.
///
/// Comportement selon le type d'action :
/// - `ban*`  : appelle Discord API pour **unban** l'utilisateur puis supprime la ligne.
/// - `mute*` / `timeout` : appelle Discord API pour **retirer le timeout**
///   (`communication_disabled_until = null`) puis supprime la ligne.
/// - `warn` / autre : supprime juste la ligne (pas d'effet Discord natif).
pub async fn delete_action(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let uuid = validation::parse_uuid("id", &id).map_err(ApiError)?;

    // Toute l'orchestration (effet Discord inverse, annulation du rappel
    // d'auto-unban, suppression) vit dans `CancelModerationActionUseCase` :
    // le service gRPC appelle le meme, il ne peut donc pas y avoir deux
    // comportements selon le transport utilise.
    use sentinel_core::ports::inbound::moderation::cancel_action::CancelOutcome;
    match state.cancel_action_uc.cancel(uuid).await? {
        CancelOutcome::Cancelled => Ok(axum::http::StatusCode::NO_CONTENT),
        CancelOutcome::NotFound => Err(ApiError(
            sentinel_core::domain::errors::DomainError::NotFound("Action introuvable".into()),
        )),
    }
}

#[derive(Deserialize)]
pub struct ModActionCountQuery {
    pub window_secs: Option<i64>,
}

/// GET /api/moderation/mod-action-count/{guild_id}/{moderator_id}?window_secs=N
///
/// Nombre d'actions de moderation posees par ce moderateur sur la fenetre
/// (defaut 3600s). Sert au garde-fou "quota par moderateur" cote bot (anti-modo
/// compromis / emballement) : le bot bloque une action au-dela du quota configure.
pub async fn mod_action_count(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path((guild_id, moderator_id)): Path<(String, String)>,
    Query(q): Query<ModActionCountQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Fenetre effective (defaut/bornes) : règle du core ; SQL : repository.
    let window =
        sentinel_core::domain::entities::moderation::action::reversal::mod_action_window_secs(
            q.window_secs,
        );
    let count = state
        .moderation_uc
        .count_recent_mod_actions(&guild_id, &moderator_id, window)
        .await?;
    Ok(Json(serde_json::json!({ "count": count })))
}
