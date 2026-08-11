use crate::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::adapters::inbound::http::dto::community::voice_channels::AddCoAdminDto;
use crate::adapters::inbound::http::dto::community::voice_channels::AddWhitelistDto;
use crate::adapters::inbound::http::dto::community::voice_channels::BanFromChannelDto;
use crate::adapters::inbound::http::dto::community::voice_channels::CreateInviteLinkDto;
use crate::adapters::inbound::http::dto::community::voice_channels::CreateThemeDto;
use crate::adapters::inbound::http::dto::community::voice_channels::CreateVoiceChannelDto;
use crate::adapters::inbound::http::dto::community::voice_channels::InviteLinkResponseDto;
use crate::adapters::inbound::http::dto::community::voice_channels::ThemeResponseDto;
use crate::adapters::inbound::http::dto::community::voice_channels::TransferOwnershipDto;
use crate::adapters::inbound::http::dto::community::voice_channels::UpdateVoiceChannelDto;
use crate::adapters::inbound::http::dto::community::voice_channels::UseInviteLinkDto;
use crate::adapters::inbound::http::dto::community::voice_channels::VoiceChannelDetailDto;
use crate::adapters::inbound::http::dto::community::voice_channels::VoiceChannelResponseDto;
use crate::adapters::inbound::http::dto::community::voice_channels::WhitelistEntryResponseDto;
use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::helpers::map_to_dtos;
use crate::adapters::inbound::http::helpers::ok_response;
use crate::adapters::inbound::http::helpers::single_dto;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::bootstrap::state::VoiceChannelsState;
use sentinel_core::domain::errors::DomainError;

/// Ensemble des guilds ou le caller est Moderator+ (pour scoper les endpoints
/// guild-less comme `list_all_channels`). Délègue au use case tickets (source
/// unique de la règle, plus de SQL dupliqué dans l'inbound).
async fn moderated_guilds(
    state: &VoiceChannelsState,
    user_id: &str,
) -> Result<std::collections::HashSet<String>, ApiError> {
    Ok(state.tickets_uc.moderated_guilds(user_id).await?)
}
use sentinel_core::domain::entities::system::discord_ids::ChannelId;
use sentinel_core::domain::entities::system::discord_ids::GuildId;
use sentinel_core::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::CreateThemeCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use sentinel_core::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;

async fn log_voice_event(
    state: &VoiceChannelsState,
    guild_id: GuildId,
    event_type: &str,
    channel_id: ChannelId,
    channel_name: Option<String>,
    actor_id: Option<String>,
    actor_name: Option<String>,
    details: serde_json::Value,
) {
    let cmd = CreateAuditLogCommand {
        guild_id,
        event_type: event_type.to_string(),
        actor_id,
        actor_name,
        target_id: None,
        target_name: None,
        channel_id: Some(channel_id.into()),
        channel_name,
        details,
    };
    if let Err(e) = state.audit_logs_uc.create(cmd).await {
        tracing::warn!("failed to log voice audit event: {e}");
    }
}

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Channels ──

mod access;
mod invites;
mod lifecycle;
mod themes;

pub use access::*;
pub use invites::*;
pub use lifecycle::*;
pub use themes::*;

#[cfg(test)]
#[path = "tests/voice_channels.rs"]
mod tests;
