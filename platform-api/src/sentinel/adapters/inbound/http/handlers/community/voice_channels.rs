use crate::sentinel::adapters::inbound::http::extractors::ValidatedGuild;
use axum::extract::Path;
use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;

use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::AddCoAdminDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::AddWhitelistDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::BanFromChannelDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::CreateInviteLinkDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::CreateThemeDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::CreateVoiceChannelDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::InviteLinkResponseDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::ThemeResponseDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::TransferOwnershipDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::UpdateVoiceChannelDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::UseInviteLinkDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::VoiceChannelDetailDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::VoiceChannelResponseDto;
use crate::sentinel::adapters::inbound::http::dto::community::voice_channels::WhitelistEntryResponseDto;
use crate::sentinel::adapters::inbound::http::errors::ApiError;
use crate::sentinel::adapters::inbound::http::helpers::map_to_dtos;
use crate::sentinel::adapters::inbound::http::helpers::ok_response;
use crate::sentinel::adapters::inbound::http::helpers::single_dto;
use crate::sentinel::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::sentinel::bootstrap::state::VoiceChannelsState;
use platform_core::sentinel::domain::errors::DomainError;

/// Ensemble des guilds ou le caller est Moderator+ (pour scoper les endpoints
/// guild-less comme `list_all_channels`). Délègue au use case tickets (source
/// unique de la règle, plus de SQL dupliqué dans l'inbound).
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::CreateAuditLogCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::BanFromChannelCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::CreateInviteLinkCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::CreateThemeCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::ManageCoAdminCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::ManageWhitelistCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::TransferOwnershipCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::UpdateVoiceChannelCommand;
use platform_core::sentinel::ports::inbound::community::manage_voice_channels::UseInviteLinkCommand;

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
