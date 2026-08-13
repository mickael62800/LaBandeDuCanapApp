use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::dto::community::confessions::{
    parse_report_status, ConfessionDto, ConfigDto, CreateConfessionDto, CreateReplyDto,
    CreateReportDto, DeleteConfessionDto, EditConfessionDto, ReplyDto, ReportDto, ResolveReportDto,
    SaveConfigDto, UpdateMessageRefsDto, UpdateReplyMessageDto,
};
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use axum::Extension;
use platform_core::sentinel::domain::entities::community::confession::{
    ConfessionConfig, ReportStatus,
};
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_confessions::{
    CreateConfessionCommand, CreateReplyCommand, CreateReportCommand,
};

#[derive(serde::Deserialize)]
pub struct ListConfessionsQuery {
    pub limit: Option<i64>,
    pub include_deleted: Option<bool>,
}

#[derive(serde::Deserialize)]
pub struct ListReportsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
}

// ── Helpers RBAC / anonymat ───────────────────────────────────────────────
//
// L'INTERET des confessions est l'anonymat : l'`author_user_id` (et le
// `reporter_user_id` des signalements) ne doit JAMAIS fuir vers un caller web
// qui n'a pas le role suffisant. On reproduit le pattern web-vs-bot deja
// applique a automod (`effective_facts`), tickets (`require_ticket_web`) et
// voice (`gate_by_channel_id`) :
//
//   - Pas de `RoleContext` (appel bot/interne, AuthKind::Internal/Bearer) =>
//     confiance totale : acces complet (le bot a besoin de l'auteur pour le
//     cooldown / la commande reveal).
//   - `RoleContext` present (caller web, X-Discord-Token) => on enforce le
//     role REEL sur la guild de la confession.

/// DTO confession avec redaction conditionnelle de `author_user_id`.
fn confession_dto(
    c: platform_core::sentinel::domain::entities::community::confession::Confession,
    redact: bool,
) -> ConfessionDto {
    let mut dto = ConfessionDto::from(c);
    if redact {
        dto.author_user_id.clear();
    }
    dto
}

fn reply_dto(
    r: platform_core::sentinel::domain::entities::community::confession::ConfessionReply,
    redact: bool,
) -> ReplyDto {
    let mut dto = ReplyDto::from(r);
    if redact {
        dto.author_user_id.clear();
    }
    dto
}

fn report_dto(
    r: platform_core::sentinel::domain::entities::community::confession::ConfessionReport,
    redact: bool,
) -> ReportDto {
    let mut dto = ReportDto::from(r);
    if redact {
        dto.reporter_user_id.clear();
    }
    dto
}

/// Identite de l'acteur a utiliser : pour un caller web on derive l'id du
/// PRINCIPAL authentifie (`ctx.discord_user_id`), en ignorant la valeur du
/// body (sinon un user pourrait forger un autre auteur / reporter, ou spoofer
/// la propriete d'une confession dans `edit_content`). Pour le bot/interne on
/// garde la valeur du body (le bot transmet le vrai soumetteur).
fn actor_id(user: &Option<Extension<WebUser>>, body_value: String) -> String {
    match user {
        Some(Extension(u)) => u.discord_user_id.clone(),
        None => body_value,
    }
}

// ── Confessions ─────────────────────────────────────────────────────────

pub async fn create_confession(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let author_user_id = actor_id(&user, dto.author_user_id);
    let c = state
        .confessions_uc
        .create(CreateConfessionCommand {
            guild_id: dto.guild_id.clone(),
            author_user_id,
            content: dto.content,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_created",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
        }),
    );
    Ok(single_dto(c))
}

pub async fn update_message_refs(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateMessageRefsDto>,
) -> Result<Json<()>, ApiError> {
    // Mutation technique (refs Discord) : gate Moderator+ pour le web ;
    // bot/interne = pass-through.
    state
        .confessions_uc
        .update_message_refs(id, dto.message_id, dto.channel_id, dto.thread_id)
        .await?;
    Ok(Json(()))
}

pub async fn edit_confession(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<EditConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    // Gate Moderator+ (web) ; ownership re-checkee par le core contre l'id
    // derive du principal.
    // S2 : pour le web, l'auteur compare est le PRINCIPAL (anti-spoof).
    let author_user_id = actor_id(&user, dto.author_user_id);
    let c = state
        .confessions_uc
        .edit_content(id, &author_user_id, dto.content)
        .await?;
    state.broadcaster.broadcast(
        "confession_edited",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
        }),
    );
    // Redaction : mutation gardee au seuil Moderator mais l'auteur n'est visible
    // qu'a partir d'Admin (cf. get/list) -> on redacte toujours ici (plus de
    // desanonymisation via edit/delete).
    Ok(Json(confession_dto(c, true)))
}

pub async fn delete_confession(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<DeleteConfessionDto>,
) -> Result<Json<ConfessionDto>, ApiError> {
    // Gate RBAC web (moderator+) : on resout la guild via la confession.
    // Appel bot (pas de RoleContext) = pass-through.
    let deleted_by = actor_id(&user, dto.deleted_by);
    let c = state
        .confessions_uc
        .delete(id, deleted_by, dto.reason)
        .await?;
    state.broadcaster.broadcast(
        "confession_deleted",
        serde_json::json!({
            "guild_id": &c.guild_id,
            "id": c.id,
            "public_number": c.public_number,
            "message_id": &c.message_id,
            "channel_id": &c.channel_id,
        }),
    );
    // Redaction (anti-desanonymisation via delete, cf. edit).
    Ok(Json(confession_dto(c, true)))
}

pub async fn get_confession(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ConfessionDto>, ApiError> {
    let c = state.confessions_uc.get(id).await?;
    // Route sans {guild_id} : on verifie l'appartenance + le role sur la guild
    // de la confession (sinon fetch cross-guild = deanon).
    Ok(Json(confession_dto(c, false)))
}

pub async fn get_by_message_id(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(message_id): Path<String>,
) -> Result<Json<Option<ConfessionDto>>, ApiError> {
    let Some(c) = state.confessions_uc.get_by_message_id(&message_id).await? else {
        return Ok(Json(None));
    };
    // Route publique par message_id : verifier appartenance a la guild de la
    // confession avant TOUT retour (sinon deanon cross-guild).
    Ok(Json(Some(confession_dto(c, false))))
}

pub async fn list_confessions(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<ListConfessionsQuery>,
) -> Result<Json<Vec<ConfessionDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let include_deleted = params.include_deleted.unwrap_or(false);
    let list = state
        .confessions_uc
        .list(&guild_id, limit, include_deleted)
        .await?;
    Ok(Json(
        list.into_iter().map(|c| confession_dto(c, false)).collect(),
    ))
}

// ── Replies ─────────────────────────────────────────────────────────────

pub async fn create_reply(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(confession_id): Path<Uuid>,
    Json(dto): Json<CreateReplyDto>,
) -> Result<Json<ReplyDto>, ApiError> {
    // Action utilisateur normale : tout membre de la guild peut repondre.
    // On verifie juste l'appartenance (web) et on derive l'auteur du principal.
    let author_user_id = actor_id(&user, dto.author_user_id);
    let r = state
        .confessions_uc
        .create_reply(CreateReplyCommand {
            confession_id,
            author_user_id,
            content: dto.content,
            is_anonymous: dto.is_anonymous,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_reply_created",
        serde_json::json!({
            "confession_id": confession_id,
            "id": r.id,
            "public_number": r.public_number,
        }),
    );
    Ok(single_dto(r))
}

pub async fn update_reply_message_id(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateReplyMessageDto>,
) -> Result<Json<()>, ApiError> {
    state
        .confessions_uc
        .update_reply_message_id(id, dto.message_id)
        .await?;
    Ok(Json(()))
}

pub async fn delete_reply(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<DeleteConfessionDto>,
) -> Result<Json<ReplyDto>, ApiError> {
    // Gate Moderator+ (web) : on resout la guild via la confession parente.
    let deleted_by = actor_id(&user, dto.deleted_by);
    let r = state.confessions_uc.delete_reply(id, deleted_by).await?;
    state.broadcaster.broadcast(
        "confession_reply_deleted",
        serde_json::json!({
            "confession_id": r.confession_id,
            "id": r.id,
            "message_id": &r.message_id,
        }),
    );
    // Redaction (anti-desanonymisation via delete_reply).
    Ok(Json(reply_dto(r, true)))
}

pub async fn list_replies(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Path(confession_id): Path<Uuid>,
) -> Result<Json<Vec<ReplyDto>>, ApiError> {
    // Route sans {guild_id} : on resout la guild via la confession parente,
    // on verifie l'appartenance, puis on redacte les auteurs sous Admin.
    state.confessions_uc.get(confession_id).await?;
    let list = state.confessions_uc.list_replies(confession_id).await?;
    Ok(Json(
        list.into_iter().map(|r| reply_dto(r, false)).collect(),
    ))
}

// ── Reports ─────────────────────────────────────────────────────────────

pub async fn create_report(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<CreateReportDto>,
) -> Result<Json<ReportDto>, ApiError> {
    // Action utilisateur normale : tout membre peut signaler. Web => verifie
    // l'appartenance a la guild ciblee et derive le reporter du principal.
    let reporter_user_id = actor_id(&user, dto.reporter_user_id);
    let r = state
        .confessions_uc
        .create_report(CreateReportCommand {
            guild_id: dto.guild_id.clone(),
            confession_id: dto.confession_id,
            reply_id: dto.reply_id,
            reporter_user_id,
            reason: dto.reason,
        })
        .await?;
    state.broadcaster.broadcast(
        "confession_report_created",
        serde_json::json!({ "guild_id": &r.guild_id, "id": r.id }),
    );
    Ok(single_dto(r))
}

pub async fn list_reports(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
    Query(params): Query<ListReportsQuery>,
) -> Result<Json<Vec<ReportDto>>, ApiError> {
    let limit = params.limit.unwrap_or(50).min(500);
    let status = params.status.as_deref().and_then(ReportStatus::from_str);
    // A5 : `reporter_user_id` reserve aux Moderateurs+ (web) ; redacte en
    // dessous. Le bot a un acces complet.
    let list = state
        .confessions_uc
        .list_reports(&guild_id, status, limit)
        .await?;
    Ok(Json(
        list.into_iter().map(|r| report_dto(r, false)).collect(),
    ))
}

pub async fn resolve_report(
    State(state): State<CommunityState>,
    user: Option<Extension<WebUser>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<ResolveReportDto>,
) -> Result<Json<()>, ApiError> {
    let status =
        parse_report_status(&dto.status).map_err(|m| ApiError(DomainError::ValidationError(m)))?;
    let resolved_by = actor_id(&user, dto.resolved_by);
    state
        .confessions_uc
        .resolve_report(id, status, resolved_by)
        .await?;
    Ok(Json(()))
}

// ── Config ──────────────────────────────────────────────────────────────

pub async fn get_config(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    ValidatedGuild { guild_id }: ValidatedGuild,
) -> Result<Json<ConfigDto>, ApiError> {
    let cfg = state.confessions_uc.get_config(&guild_id).await?;
    Ok(single_dto(cfg))
}

pub async fn save_config(
    State(state): State<CommunityState>,
    _user: Option<Extension<WebUser>>,
    Json(dto): Json<SaveConfigDto>,
) -> Result<Json<ConfigDto>, ApiError> {
    let cfg = ConfessionConfig {
        guild_id: dto.guild_id,
        enabled: dto.enabled,
        channel_id: dto.channel_id,
        panel_message_id: dto.panel_message_id,
        cooldown_secs: dto.cooldown_secs,
        max_per_day: dto.max_per_day,
        quota_window_hours: dto.quota_window_hours,
        min_chars: dto.min_chars,
        max_chars: dto.max_chars,
        automod_enabled: dto.automod_enabled,
        banned_user_ids: dto.banned_user_ids,
        updated_at: chrono::Utc::now(),
    };
    let saved = state.confessions_uc.save_config(cfg).await?;
    Ok(single_dto(saved))
}
