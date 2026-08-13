use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::dto::community::announcements::{
    parse_content_type, parse_recurrence, AnnouncementDto, AnnouncementRunDto, ButtonClickDto,
    ButtonInteractionDto, CreateAnnouncementDto, RecordRunResultDto, ToggleAnnouncementDto,
    UpdateAnnouncementDto,
};
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use axum::Extension;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_announcements::{
    CreateAnnouncementCommand, RenderedAnnouncement, UpdateAnnouncementCommand,
};

const ANNOUNCEMENTS_BOT: &str = "announcements";

fn map_validation_string<T>(r: Result<T, String>) -> Result<T, ApiError> {
    r.map_err(|m| ApiError(DomainError::ValidationError(m)))
}

// ── Helpers config ──────────────────────────────────────────────────────

async fn read_cfg(state: &CommunityState, guild_id: &str, key: &str) -> Option<String> {
    let cfgs = state
        .bot_config_repo
        .get_config(guild_id, ANNOUNCEMENTS_BOT)
        .await
        .ok()?;
    cfgs.into_iter()
        .find(|c| c.config_key == key)
        .map(|c| c.config_value)
}

fn parse_i64(v: Option<String>, default: i64) -> i64 {
    v.and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// Parse "#5865f2" ou "5865f2" → 0x5865f2 (i32). Renvoie None si invalide.
fn parse_hex_color(s: &str) -> Option<i32> {
    let s = s.trim().trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    i32::from_str_radix(s, 16).ok()
}

/// Poste un embed de log dans `log_channel_id` (best-effort, ne fait rien
/// si pas de salon configure ni de bot token).
async fn try_post_log_embed(
    state: &CommunityState,
    guild_id: &str,
    title: &str,
    description: &str,
    color: i32,
) {
    let channel_id = match read_cfg(state, guild_id, "log_channel_id").await {
        Some(s) if !s.is_empty() => s,
        _ => return,
    };
    let embed = serde_json::json!({
        "title": title,
        "description": description,
        "color": color,
        "timestamp": Utc::now().to_rfc3339(),
    });
    // Best-effort : un log qui ne part pas ne doit pas faire echouer l'action
    // qu'il documente. La validation du salon et l'absence de token sont
    // gerees par l'adaptateur, qui rend une erreur au lieu d'appeler Discord.
    if let Err(e) = state
        .discord_api
        .send_channel_embed(&channel_id, embed)
        .await
    {
        tracing::warn!(error = %e, guild = %guild_id, "log_channel_id post echec");
    }
}

pub async fn create_announcement(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateAnnouncementDto>,
) -> Result<Json<AnnouncementDto>, ApiError> {
    let recurrence_type = map_validation_string(parse_recurrence(&dto.recurrence_type))?;
    let content_type = map_validation_string(parse_content_type(&dto.content_type))?;

    // ── Cap par guild (max_announcements_per_guild) ───────────────────
    let max_per_guild = parse_i64(
        read_cfg(&state, &dto.guild_id, "max_announcements_per_guild").await,
        100,
    );
    let existing = state
        .announcements_uc
        .list_by_guild(&dto.guild_id)
        .await?
        .len() as i64;
    if existing >= max_per_guild {
        return Err(ApiError(DomainError::Conflict(format!(
            "Limite d'annonces atteinte pour ce serveur ({existing}/{max_per_guild}). Supprimez-en avant d'en creer une nouvelle."
        ))));
    }

    // ── Couleur par defaut (default_color_hex) ────────────────────────
    let embed_color = match dto.embed_color {
        Some(c) => Some(c),
        None => read_cfg(&state, &dto.guild_id, "default_color_hex")
            .await
            .as_deref()
            .and_then(parse_hex_color),
    };

    // L'auteur enregistre est le user web authentifie (WebUser). Appel
    // bot/interne (sans WebUser) : fallback "web".
    let created_by = user
        .as_ref()
        .map(|Extension(ctx)| ctx.discord_user_id.clone())
        .unwrap_or_else(|| "web".to_string());

    let guild_id_for_log = dto.guild_id.clone();
    let cmd = CreateAnnouncementCommand {
        guild_id: dto.guild_id,
        name: dto.name,
        recurrence_type,
        recurrence_hour: dto.recurrence_hour,
        recurrence_minute: dto.recurrence_minute,
        recurrence_day_of_week: dto.recurrence_day_of_week,
        recurrence_day_of_month: dto.recurrence_day_of_month,
        recurrence_month: dto.recurrence_month,
        scheduled_at: dto.scheduled_at,
        end_date: dto.end_date,
        content_type,
        content_text: dto.content_text,
        embed_title: dto.embed_title,
        embed_color,
        embed_image_url: dto.embed_image_url,
        embed_thumbnail_url: dto.embed_thumbnail_url,
        embed_footer_text: dto.embed_footer_text,
        mention_everyone: dto.mention_everyone,
        mention_here: dto.mention_here,
        mention_role_ids: dto.mention_role_ids,
        channel_ids: dto.channel_ids,
        buttons: dto.buttons,
        auto_reactions: dto.auto_reactions,
        created_by,
    };
    let ann = state.announcements_uc.create(cmd).await?;

    // Log best-effort (log_channel_id)
    try_post_log_embed(
        &state,
        &guild_id_for_log,
        "Annonce creee",
        &format!(
            "**{}** — prochaine execution : {}",
            ann.name,
            ann.next_run_at.format("%Y-%m-%d %H:%M UTC")
        ),
        0x57F287,
    )
    .await;

    Ok(single_dto(ann))
}

pub async fn update_announcement(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateAnnouncementDto>,
) -> Result<Json<AnnouncementDto>, ApiError> {
    // Fail-closed : charge l'annonce (erreur DB propagee au lieu d'etre avalee)
    // et gate toujours (check_role_for_guild bypasse en interne si pas de
    // WebUser = appel bot). Avant, `if let Ok` sautait la garde sur erreur DB.
    state.announcements_uc.get(id).await?;
    let recurrence_type = map_validation_string(parse_recurrence(&dto.recurrence_type))?;
    let content_type = map_validation_string(parse_content_type(&dto.content_type))?;
    let cmd = UpdateAnnouncementCommand {
        id,
        name: dto.name,
        recurrence_type,
        recurrence_hour: dto.recurrence_hour,
        recurrence_minute: dto.recurrence_minute,
        recurrence_day_of_week: dto.recurrence_day_of_week,
        recurrence_day_of_month: dto.recurrence_day_of_month,
        recurrence_month: dto.recurrence_month,
        scheduled_at: dto.scheduled_at,
        end_date: dto.end_date,
        content_type,
        content_text: dto.content_text,
        embed_title: dto.embed_title,
        embed_color: dto.embed_color,
        embed_image_url: dto.embed_image_url,
        embed_thumbnail_url: dto.embed_thumbnail_url,
        embed_footer_text: dto.embed_footer_text,
        mention_everyone: dto.mention_everyone,
        mention_here: dto.mention_here,
        mention_role_ids: dto.mention_role_ids,
        channel_ids: dto.channel_ids,
        buttons: dto.buttons,
        auto_reactions: dto.auto_reactions,
    };
    let ann = state.announcements_uc.update(cmd).await?;
    Ok(single_dto(ann))
}

pub async fn delete_announcement(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<()>, ApiError> {
    // Fail-closed (cf. update) : erreur DB propagee, garde toujours executee.
    state.announcements_uc.get(id).await?;
    state.announcements_uc.delete(id).await?;
    Ok(Json(()))
}

pub async fn get_announcement(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<AnnouncementDto>, ApiError> {
    let ann = state.announcements_uc.get(id).await?;
    // IDOR : sans garde, tout appelant lisait le contenu d'une annonce d'un autre
    // serveur par enumeration d'UUID.
    Ok(single_dto(ann))
}

pub async fn list_announcements(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<Vec<AnnouncementDto>>, ApiError> {
    let list = state.announcements_uc.list_by_guild(&guild_id).await?;
    Ok(map_to_dtos(list))
}

pub async fn toggle_announcement(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ToggleAnnouncementDto>,
) -> Result<Json<bool>, ApiError> {
    // Fail-closed (cf. update) : erreur DB propagee, garde toujours executee.
    state.announcements_uc.get(id).await?;
    let new_state = state.announcements_uc.toggle(id, dto.enabled).await?;
    Ok(Json(new_state))
}

pub async fn preview_announcement(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<RenderedAnnouncement>, ApiError> {
    state.announcements_uc.get(id).await?;
    let rendered = state.announcements_uc.preview(id).await?;
    Ok(Json(rendered))
}

pub async fn list_runs(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Query(params): Query<RunsLimitQuery>,
) -> Result<Json<Vec<AnnouncementRunDto>>, ApiError> {
    state.announcements_uc.get(id).await?;
    let limit = params.limit.unwrap_or(50).min(500);
    let runs = state.announcements_uc.list_runs(id, limit).await?;
    Ok(map_to_dtos(runs))
}

#[derive(serde::Deserialize)]
pub struct RunsLimitQuery {
    pub limit: Option<i64>,
}

// ── Endpoints internes worker / bot ─────────────────────────────────────

#[derive(serde::Deserialize)]
pub struct FetchDueQuery {
    pub limit: Option<i64>,
}

/// GET /api/announcements/internal/due — appele par announcement-worker.
/// Retourne les annonces dues, cree les runs (status=pending) et avance
/// next_run_at de chaque annonce. Le bot consume ensuite via Redis stream
/// et appelle /runs/{id}/result une fois le post fait.
///
/// Post-traitement : pour chaque embed sans couleur explicite, applique
/// la couleur par defaut du guild (`default_color_hex`).
pub async fn fetch_due(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Query(params): Query<FetchDueQuery>,
) -> Result<Json<Vec<RenderedAnnouncement>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(200);
    Ok(Json(prepare_due(&state, limit).await?))
}

/// POST /api/announcements/internal/jobs/publish-due
///
/// Prepare les annonces dues puis les publie sur le bus consomme par le bot.
pub async fn job_publish_due(
    State(state): State<CommunityState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let limit = std::env::var("ANNOUNCEMENTS_FETCH_LIMIT_GLOBAL")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(200)
        .clamp(1, 200);
    let payloads = prepare_due(&state, limit).await?;
    let processed = payloads.len();
    for payload in payloads {
        state.broadcaster.broadcast(
            "announcement_publish",
            serde_json::to_value(payload).map_err(|error| {
                ApiError(DomainError::Internal(format!(
                    "serialisation annonce preparee: {error}"
                )))
            })?,
        );
    }
    Ok(Json(serde_json::json!({
        "job": "announcements_publish_due",
        "processed": processed,
        "errors": 0,
    })))
}

async fn prepare_due(
    state: &CommunityState,
    limit: i64,
) -> Result<Vec<RenderedAnnouncement>, ApiError> {
    let mut payloads = state
        .announcements_uc
        .fetch_due_and_prepare(Utc::now(), limit)
        .await?;

    // Cache des defaults par guild pour eviter N requetes config.
    use std::collections::HashMap;
    let mut color_by_guild: HashMap<String, Option<i32>> = HashMap::new();
    for p in payloads.iter_mut() {
        let needs_color = p.embed.as_ref().is_some_and(|e| e.color.is_none());
        if !needs_color {
            continue;
        }
        let default = match color_by_guild.get(&p.guild_id) {
            Some(c) => *c,
            None => {
                let c = read_cfg(state, &p.guild_id, "default_color_hex")
                    .await
                    .as_deref()
                    .and_then(parse_hex_color);
                color_by_guild.insert(p.guild_id.clone(), c);
                c
            }
        };
        if let (Some(embed), Some(c)) = (p.embed.as_mut(), default) {
            embed.color = Some(c);
        }
    }

    Ok(payloads)
}

/// POST /api/announcements/internal/retention-cleanup — appele par le
/// sentinel-worker (job analytics_retention) pour purger les `announcement_runs`
/// plus vieux que `history_retention_days` (defaut 90j) par guild. Si la
/// cle est 0, la guild est skip (illimite).
pub async fn retention_cleanup_all(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let summary = state.announcements_uc.retention_cleanup_all().await?;
    Ok(Json(serde_json::json!({
        "guilds_processed": summary.guilds_processed,
        "guilds_skipped": summary.guilds_skipped,
        "rows_deleted": summary.rows_deleted,
        "status": "ok",
    })))
}

/// POST /api/announcements/internal/runs/{run_id}/result — appele par
/// le bot apres publication des messages Discord.
pub async fn record_run_result(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(run_id): Path<Uuid>,
    Json(dto): Json<RecordRunResultDto>,
) -> Result<Json<()>, ApiError> {
    state
        .announcements_uc
        .record_run_result(run_id, dto.channels_posted)
        .await?;
    Ok(Json(()))
}

/// POST /api/announcements/internal/button-click — appele par le bot
/// quand un user clique sur un bouton interactif d'une annonce.
pub async fn record_button_click(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<ButtonClickDto>,
) -> Result<Json<()>, ApiError> {
    state
        .announcements_uc
        .record_button_interaction(
            dto.announcement_id,
            dto.run_id,
            dto.user_id,
            dto.user_name,
            dto.button_custom_id,
            dto.button_label,
        )
        .await?;
    Ok(Json(()))
}

/// GET /api/announcements/{id}/interactions — liste des clics sur les boutons.
pub async fn list_button_interactions(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Query(params): Query<RunsLimitQuery>,
) -> Result<Json<Vec<ButtonInteractionDto>>, ApiError> {
    let limit = params.limit.unwrap_or(100).min(1000);
    let interactions = state
        .announcements_uc
        .list_button_interactions(id, limit)
        .await?;
    Ok(map_to_dtos(interactions))
}
