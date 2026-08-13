//! Embed builder (style Carl-bot) — CRUD + post/edit.
//!
//! Poster/editer passe par le bot : l'API publie un event `embed_publish` sur
//! le stream Redis `sentinel:events`, que le bot consomme pour poster/editer le
//! message Discord, puis rapporte l'id via `POST /api/embeds/{id}/posted`.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use redis::AsyncCommands;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::dto::community::embeds::{
    EmbedDto, EmbedInputDto, EmbedPostedDto, PostEmbedDto,
};
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::CommunityState;
use platform_core::sentinel::domain::entities::community::embed::RenderedEmbedPost;

const EMBED_STREAM_KEY: &str = "sentinel:events";
const EMBED_STREAM_MAXLEN: usize = 10_000;

/// Publie un `embed_publish` sur le stream Redis pour que le bot poste/edite.
async fn publish_embed(
    state: &CommunityState,
    payload: &RenderedEmbedPost,
) -> Result<(), ApiError> {
    let envelope = serde_json::json!({ "event": "embed_publish", "data": payload }).to_string();
    let mut conn = state
        .redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| {
            ApiError(
                platform_core::sentinel::domain::errors::DomainError::Internal(format!(
                    "Redis indisponible: {e}"
                )),
            )
        })?;
    let _: String = conn
        .xadd_maxlen(
            EMBED_STREAM_KEY,
            redis::streams::StreamMaxlen::Approx(EMBED_STREAM_MAXLEN),
            "*",
            &[("payload", envelope)],
        )
        .await
        .map_err(|e| {
            ApiError(
                platform_core::sentinel::domain::errors::DomainError::Internal(format!(
                    "XADD embed_publish: {e}"
                )),
            )
        })?;
    Ok(())
}

pub async fn list_embeds(
    State(state): State<CommunityState>,
    Path(guild_id): Path<String>,
) -> Result<Json<Vec<EmbedDto>>, ApiError> {
    let list = state.embeds_uc.list_by_guild(&guild_id).await?;
    Ok(Json(list.into_iter().map(EmbedDto::from).collect()))
}

pub async fn get_embed(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EmbedDto>, ApiError> {
    let e = state.embeds_uc.get(id).await?;
    Ok(Json(EmbedDto::from(e)))
}

pub async fn create_embed(
    State(state): State<CommunityState>,
    Path(guild_id): Path<String>,
    user: Option<Extension<WebUser>>,
    Json(dto): Json<EmbedInputDto>,
) -> Result<Json<EmbedDto>, ApiError> {
    let created_by = user
        .as_ref()
        .map(|Extension(ctx)| ctx.discord_user_id.clone())
        .unwrap_or_else(|| "web".to_string());
    let e = state
        .embeds_uc
        .create(&guild_id, &created_by, dto.into())
        .await?;
    Ok(Json(EmbedDto::from(e)))
}

pub async fn update_embed(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<EmbedInputDto>,
) -> Result<Json<EmbedDto>, ApiError> {
    let e = state.embeds_uc.update(id, dto.into()).await?;
    Ok(Json(EmbedDto::from(e)))
}

pub async fn delete_embed(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.embeds_uc.delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// POST /api/embeds/{id}/post — poste l'embed dans un salon (nouveau message).
pub async fn post_embed(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<PostEmbedDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let payload = state.embeds_uc.prepare_post(id, &dto.channel_id).await?;
    publish_embed(&state, &payload).await?;
    Ok(Json(serde_json::json!({ "queued": true, "mode": "post" })))
}

/// POST /api/embeds/{id}/edit — re-edite le dernier message poste de cet embed.
pub async fn edit_embed(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let payload = state.embeds_uc.prepare_edit(id).await?;
    publish_embed(&state, &payload).await?;
    Ok(Json(serde_json::json!({ "queued": true, "mode": "edit" })))
}

/// POST /api/embeds/{id}/posted — rapport du bot apres un post reussi.
pub async fn record_embed_posted(
    State(state): State<CommunityState>,
    Path(id): Path<Uuid>,
    Json(dto): Json<EmbedPostedDto>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state
        .embeds_uc
        .record_posted(id, &dto.channel_id, &dto.message_id)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
