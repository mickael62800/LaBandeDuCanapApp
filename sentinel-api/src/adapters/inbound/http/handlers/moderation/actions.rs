use crate::adapters::inbound::http::extractors::{ValidatedGuild, ValidatedGuildUser};
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::moderation::actions::BanEntryDto;
use crate::adapters::inbound::http::dto::moderation::actions::LogActionDto;
use crate::adapters::inbound::http::dto::moderation::actions::ModerationActionResponseDto;
use crate::adapters::inbound::http::dto::moderation::actions::UserHistoryDto;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::adapters::inbound::http::validation;
use crate::bootstrap::state::ModerationState;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::domain::entities::system::discord_ids::UserId;

/// S1/S4 — Resout l'identite moderateur a journaliser.
///
/// Web (WebUser present, token Discord verifie) -> identite authentifiee,
/// les valeurs eventuellement fournies dans le body sont ignorees. Interne
/// (pas de WebUser : gRPC/Bearer/desktop) -> valeurs par defaut fournies.
fn resolve_web_moderator(
    user: &Option<Extension<WebUser>>,
    default_id: &str,
    default_name: &str,
) -> (String, String) {
    match user {
        Some(Extension(ctx)) => (ctx.discord_user_id.clone(), ctx.discord_user_id.clone()),
        None => (default_id.to_string(), default_name.to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct BansQuery {
    pub guild_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// POST /api/moderation/actions — enregistrer une action de modération
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
    /// Nom d'affichage optionnel (stocke dans moderation_actions.target_name).
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

/// GET /api/moderation/bans
pub async fn list_bans(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<BansQuery>,
) -> Result<Json<Vec<BanEntryDto>>, ApiError> {
    // Validation
    validation::validate_optional_discord_id("guild_id", &params.guild_id).map_err(ApiError)?;
    validation::validate_pagination(params.limit, params.offset).map_err(ApiError)?;

    let limit = crate::adapters::inbound::http::helpers::normalize_limit(params.limit, 50, 500);
    let offset = crate::adapters::inbound::http::helpers::normalize_offset(params.offset);
    let bans = state
        .moderation_uc
        .list_bans(params.guild_id.as_deref(), limit, offset)
        .await?;
    Ok(map_to_dtos(bans))
}

/// GET /api/moderation/history/{guild_id}/{user_id}
pub async fn get_history(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuildUser { guild_id, user_id }: ValidatedGuildUser,
) -> Result<Json<UserHistoryDto>, ApiError> {
    // Validation

    let history = state.moderation_uc.get_history(&guild_id, &user_id).await?;
    Ok(single_dto(history))
}

/// MOD #2 — POST /api/moderation/evidence
///
/// Attache une preuve (URL + description optionnelle) a une action de moderation
/// existante. La FK assure qu'on ne peut pas attacher a une action inconnue.
#[derive(Debug, serde::Deserialize)]
pub struct AddEvidenceDto {
    pub action_id: String,
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
}

#[derive(Debug, serde::Serialize)]
pub struct EvidenceEntryDto {
    pub id: String,
    pub action_id: String,
    pub url: String,
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
    pub uploaded_at: String,
}

pub async fn add_evidence(
    State(state): State<ModerationState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<AddEvidenceDto>,
) -> Result<Json<EvidenceEntryDto>, ApiError> {
    // Pour gater user on a besoin du guild_id : on le recupere via l'action liee.
    if user.is_some() {
        if let Ok(_action_uuid) = uuid::Uuid::parse_str(&dto.action_id) {}
    }
    // Validation URL — regle metier dans `domain/entities/moderation_review.rs`.
    sentinel_core::domain::entities::moderation::review::manual::validate_evidence_url(&dto.url)
        .map_err(|m| {
            ApiError(sentinel_core::domain::errors::DomainError::ValidationError(
                m.into(),
            ))
        })?;
    let action_uuid = validation::parse_uuid("action_id", &dto.action_id).map_err(ApiError)?;
    validation::validate_discord_id("uploaded_by", &dto.uploaded_by).map_err(ApiError)?;
    let description = dto
        .description
        .as_deref()
        .map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let entry = state
        .evidence_repo
        .add(
            action_uuid,
            &dto.url,
            description.as_deref(),
            &dto.uploaded_by,
            &dto.uploaded_by_name,
        )
        .await?;

    Ok(Json(EvidenceEntryDto {
        id: entry.id.to_string(),
        action_id: dto.action_id,
        url: entry.url,
        description: entry.description,
        uploaded_by: dto.uploaded_by,
        uploaded_by_name: dto.uploaded_by_name,
        uploaded_at: entry.uploaded_at.to_rfc3339(),
    }))
}

/// MOD #2 — GET /api/moderation/evidence/{action_id}
///
/// Liste les preuves attachees a une action.
pub async fn list_evidence(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Path(action_id): Path<String>,
) -> Result<Json<Vec<EvidenceEntryDto>>, ApiError> {
    let action_uuid = validation::parse_uuid("action_id", &action_id).map_err(ApiError)?;

    let entries = state.evidence_repo.list(action_uuid).await?;
    let dtos = entries
        .into_iter()
        .map(|e| EvidenceEntryDto {
            id: e.id.to_string(),
            action_id: action_id.clone(),
            url: e.url,
            description: e.description,
            uploaded_by: e.uploaded_by,
            uploaded_by_name: e.uploaded_by_name,
            uploaded_at: e.uploaded_at.to_rfc3339(),
        })
        .collect();

    Ok(Json(dtos))
}

/// MOD #3 — POST /api/moderation/review
#[derive(Debug, serde::Deserialize)]
pub struct AddReviewDto {
    pub action_id: String,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct ReviewQueueEntryDto {
    pub id: String,
    pub action_id: String,
    pub guild_id: GuildId,
    pub added_by: String,
    pub added_by_name: String,
    pub reason: Option<String>,
    pub status: String,
    pub reviewer_id: Option<String>,
    pub reviewer_name: Option<String>,
    pub reviewer_notes: Option<String>,
    pub added_at: String,
    pub resolved_at: Option<String>,
    // Enrichissement : infos de l'action liee
    pub action_type: Option<String>,
    pub target_name: Option<String>,
    pub action_reason: Option<String>,
}

fn review_entry_to_dto(
    e: sentinel_core::ports::outbound::moderation::review_repository::ReviewEntry,
) -> ReviewQueueEntryDto {
    ReviewQueueEntryDto {
        id: e.id.to_string(),
        action_id: e.action_id.to_string(),
        guild_id: e.guild_id,
        added_by: e.added_by,
        added_by_name: e.added_by_name,
        reason: e.reason,
        status: e.status,
        reviewer_id: e.reviewer_id,
        reviewer_name: e.reviewer_name,
        reviewer_notes: e.reviewer_notes,
        added_at: e.added_at.to_rfc3339(),
        resolved_at: e.resolved_at.map(|d| d.to_rfc3339()),
        action_type: e.action_type,
        target_name: e.target_name,
        action_reason: e.action_reason,
    }
}

pub async fn add_review(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<AddReviewDto>,
) -> Result<Json<ReviewQueueEntryDto>, ApiError> {
    let action_uuid = validation::parse_uuid("action_id", &dto.action_id).map_err(ApiError)?;
    validation::validate_discord_id("guild_id", &dto.guild_id).map_err(ApiError)?;
    validation::validate_discord_id("added_by", &dto.added_by).map_err(ApiError)?;

    let reason = dto
        .reason
        .as_deref()
        .map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let entry = state
        .review_repo
        .add(
            action_uuid,
            &dto.guild_id,
            &dto.added_by,
            &dto.added_by_name,
            reason.as_deref(),
        )
        .await?;

    Ok(Json(review_entry_to_dto(entry)))
}

/// MOD #3 — GET /api/moderation/review/{guild_id}/pending
///
/// Liste les reviews en attente pour une guild, enrichies avec les infos de
/// l'action de moderation liee (JOIN avec moderation_actions).
pub async fn list_pending_reviews(
    State(state): State<ModerationState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<ReviewQueueEntryDto>>, ApiError> {
    let entries = state.review_repo.list_pending(&guild_id).await?;
    Ok(Json(entries.into_iter().map(review_entry_to_dto).collect()))
}

/// MOD #3 — PATCH /api/moderation/review/{id}/resolve
#[derive(Debug, serde::Deserialize)]
pub struct ResolveReviewDto {
    pub status: String,
    pub reviewer_id: String,
    pub reviewer_name: String,
    #[serde(default)]
    pub reviewer_notes: Option<String>,
}

pub async fn resolve_review(
    State(state): State<ModerationState>,
    // TODO(secu) : aucun gate par role ici. La protection vient uniquement des
    // middlewares du routeur (auth Bearer + superadmin + guild_auth). Le
    // controle fin � Moderator+ sur CETTE guilde � reste a implementer � il
    // existait sous forme d'un `if user.is_some() {}` vide, qui ne verifiait
    // rien tout en donnant l'impression du contraire.
    _user: Option<Extension<WebUser>>,
    Path(id): Path<String>,
    Json(dto): Json<ResolveReviewDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let review_uuid = validation::parse_uuid("id", &id).map_err(ApiError)?;

    if !sentinel_core::domain::entities::moderation::review::manual::is_valid_review_status(
        &dto.status,
    ) {
        return Err(ApiError(
            sentinel_core::domain::errors::DomainError::ValidationError(
                "status doit etre approved/rejected/changed".into(),
            ),
        ));
    }
    validation::validate_discord_id("reviewer_id", &dto.reviewer_id).map_err(ApiError)?;
    let notes = dto
        .reviewer_notes
        .as_deref()
        .map(sentinel_core::domain::entities::moderation::review::manual::truncate_review_text);

    let resolved = state
        .review_repo
        .resolve(
            review_uuid,
            &dto.reviewer_id,
            &dto.reviewer_name,
            notes.as_deref(),
            &dto.status,
        )
        .await?;

    if !resolved {
        return Err(ApiError(
            sentinel_core::domain::errors::DomainError::NotFound(
                "review introuvable ou deja resolue".into(),
            ),
        ));
    }

    Ok(Json(serde_json::json!({ "ok": true })))
}

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

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;


