//! Handlers HTTP pour la table `discord_action_messages` (cf. SYNC_DESIGN
//! phase 1). Permet au bot d'enregistrer les messages qu'il poste, et au
//! reste de l'API de retrouver `(channel_id, message_id)` pour edit.

use axum::extract::Path;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::bootstrap::state::AuditState;
use platform_core::sentinel::domain::entities::audit::discord_action_message::DiscordActionMessage;
use platform_core::sentinel::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
#[derive(Debug, Deserialize)]
pub struct RegisterDto {
    pub action_id: Uuid,
    pub kind: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
}

#[derive(Debug, Serialize)]
pub struct DiscordActionMessageDto {
    pub action_id: Uuid,
    pub kind: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub posted_at: String,
    pub last_edited_at: Option<String>,
}

impl From<DiscordActionMessage> for DiscordActionMessageDto {
    fn from(m: DiscordActionMessage) -> Self {
        Self {
            action_id: m.action_id,
            kind: m.kind,
            guild_id: m.guild_id,
            channel_id: m.channel_id,
            message_id: m.message_id,
            posted_at: m.posted_at.to_rfc3339(),
            last_edited_at: m.last_edited_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// POST /api/discord-messages/register
pub async fn register(
    State(state): State<AuditState>,
    Json(dto): Json<RegisterDto>,
) -> Result<StatusCode, ApiError> {
    state
        .discord_action_messages_uc
        .register(NewDiscordActionMessage {
            action_id: dto.action_id,
            kind: dto.kind,
            guild_id: dto.guild_id,
            channel_id: dto.channel_id,
            message_id: dto.message_id,
        })
        .await?;
    Ok(StatusCode::CREATED)
}

/// GET /api/discord-messages/{action_id}
pub async fn list_for_action(
    State(state): State<AuditState>,
    Path(action_id): Path<Uuid>,
) -> Result<Json<Vec<DiscordActionMessageDto>>, ApiError> {
    let list = state
        .discord_action_messages_uc
        .list_for_action(action_id)
        .await?;
    Ok(Json(
        list.into_iter()
            .map(DiscordActionMessageDto::from)
            .collect(),
    ))
}
