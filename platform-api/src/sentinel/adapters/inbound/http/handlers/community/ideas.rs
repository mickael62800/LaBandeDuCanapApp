//! Boite a idees — CRUD + decision du staff.
//!
//! Le bot appelle ces routes en HTTP (creation depuis la modale, sync des
//! messages du salon). Quand la decision vient du **web**, l'API publie un
//! event `idea_decided` sur le stream Redis `sentinel:events` : le bot met a
//! jour l'embed du salon de l'idee et previent l'auteur.

use axum::extract::{Path, Query, State};
use axum::{Extension, Json};
use redis::AsyncCommands;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::dto::community::ideas::{
    AddIdeaMessageDto, CreateIdeaDto, DecideIdeaDto, IdeaDetailDto, IdeaDto, IdeaMessageDto,
    ListIdeasQuery, SetIdeaChannelDto,
};
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::community::manage_ideas::{
    AddIdeaMessageCommand, CreateIdeaCommand, DecideIdeaCommand,
};
use platform_core::sentinel::ports::outbound::community::idea_repository::IdeaFilters;

const IDEA_STREAM_KEY: &str = "sentinel:events";
const IDEA_STREAM_MAXLEN: usize = 10_000;

const DEFAULT_LIMIT: i64 = 50;

/// Previent le bot qu'une idee a change de statut depuis le web.
async fn publish_decision(state: &CommunityState, idea: &IdeaDto) -> Result<(), ApiError> {
    let envelope = serde_json::json!({
        "event": "idea_decided",
        "data": {
            "idea_id": idea.id,
            "guild_id": idea.guild_id,
            "channel_id": idea.channel_id,
            "title": idea.title,
            "status": idea.status,
            "author_id": idea.author_id,
            "decided_by_name": idea.decided_by_name,
            "reason": idea.decision_reason,
        }
    })
    .to_string();

    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("Redis indisponible: {e}"))))?;
    let _: String = conn
        .xadd_maxlen(
            IDEA_STREAM_KEY,
            redis::streams::StreamMaxlen::Approx(IDEA_STREAM_MAXLEN),
            "*",
            &[("payload", envelope)],
        )
        .await
        .map_err(|e| ApiError(DomainError::Internal(format!("XADD idea_decided: {e}"))))?;
    Ok(())
}

pub async fn list_ideas(
    State(state): State<CommunityState>,
    Query(q): Query<ListIdeasQuery>,
) -> Result<Json<Vec<IdeaDto>>, ApiError> {
    let filters = IdeaFilters {
        guild_id: q.guild_id.as_deref(),
        status: q.status.as_deref(),
        category: q.category.as_deref(),
        author_id: q.author_id.as_deref(),
        search: q.search.as_deref(),
    };
    let list = state
        .ideas_uc
        .list(
            filters,
            q.limit.unwrap_or(DEFAULT_LIMIT),
            q.offset.unwrap_or(0),
        )
        .await?;
    Ok(Json(list.into_iter().map(IdeaDto::from).collect()))
}

pub async fn get_idea(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
) -> Result<Json<IdeaDetailDto>, ApiError> {
    let detail = state.ideas_uc.get_detail(id).await?;
    Ok(Json(IdeaDetailDto::from(detail)))
}

/// GET /api/ideas/by-channel/{channel_id} — utilise par le bot pour retrouver
/// l'idee attachee au salon d'ou vient une interaction ou un message.
pub async fn get_idea_by_channel(
    State(state): State<CommunityState>,
    Path(channel_id): Path<String>,
) -> Result<Json<IdeaDto>, ApiError> {
    let idea = state
        .ideas_uc
        .get_by_channel(&channel_id)
        .await?
        .ok_or_else(|| {
            ApiError(DomainError::NotFound(format!(
                "Aucune idee pour le salon {channel_id}"
            )))
        })?;
    Ok(Json(IdeaDto::from(idea)))
}

pub async fn create_idea(
    State(state): State<CommunityState>,
    Json(dto): Json<CreateIdeaDto>,
) -> Result<Json<IdeaDto>, ApiError> {
    let idea = state
        .ideas_uc
        .create(CreateIdeaCommand {
            guild_id: dto.guild_id,
            title: dto.title,
            description: dto.description,
            category: dto.category,
            author_id: dto.author_id,
            author_name: dto.author_name,
            channel_id: dto.channel_id,
        })
        .await?;
    Ok(Json(IdeaDto::from(idea)))
}

/// PATCH /api/ideas/{id}/status — decision du staff (bot ou web).
pub async fn decide_idea(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<DecideIdeaDto>,
) -> Result<Json<IdeaDto>, ApiError> {
    // Depuis le web, l'identite vient de la session : un client ne peut pas
    // s'attribuer la decision de quelqu'un d'autre.
    let (decided_by, decided_by_name) = match user.as_ref() {
        Some(Extension(ctx)) => (
            ctx.discord_user_id.clone(),
            // La session ne porte que l'id : le pseudo reste indicatif.
            dto.decided_by_name
                .clone()
                .unwrap_or_else(|| ctx.discord_user_id.clone()),
        ),
        None => (
            dto.decided_by.unwrap_or_else(|| "bot".to_string()),
            dto.decided_by_name.unwrap_or_else(|| "Staff".to_string()),
        ),
    };
    let from_web = user.is_some();

    let idea = state
        .ideas_uc
        .decide(DecideIdeaCommand {
            id,
            status: dto.status,
            decided_by,
            decided_by_name,
            reason: dto.reason,
        })
        .await?;
    let dto = IdeaDto::from(idea);

    // Le bot a deja mis a jour Discord quand la decision vient de lui : on ne
    // republie que pour les decisions prises depuis le web.
    if from_web {
        publish_decision(&state, &dto).await?;
    }
    Ok(Json(dto))
}

/// PATCH /api/ideas/{id}/channel — rattache le salon cree par le bot.
pub async fn set_idea_channel(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SetIdeaChannelDto>,
) -> Result<Json<IdeaDto>, ApiError> {
    let idea = state
        .ideas_uc
        .set_channel(id, dto.channel_id.as_deref())
        .await?;
    Ok(Json(IdeaDto::from(idea)))
}

/// POST /api/ideas/{id}/messages — sync d'un message du salon de l'idee.
pub async fn add_idea_message(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<AddIdeaMessageDto>,
) -> Result<Json<IdeaMessageDto>, ApiError> {
    let message = state
        .ideas_uc
        .add_message(AddIdeaMessageCommand {
            idea_id: id,
            author_name: dto.author_name,
            author_role: dto.author_role,
            content: dto.content,
        })
        .await?;
    Ok(Json(IdeaMessageDto::from(message)))
}

pub async fn delete_idea(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.ideas_uc.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// GET /api/ideas/quota/{guild_id}/{author_id} — nombre d'idees non tranchees,
/// consulte par le bot avant d'ouvrir une nouvelle proposition.
pub async fn get_idea_quota(
    State(state): State<CommunityState>,
    Path((guild_id, author_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let count = state
        .ideas_uc
        .count_open_by_author(&guild_id, &author_id)
        .await?;
    Ok(Json(serde_json::json!({ "open_count": count })))
}
