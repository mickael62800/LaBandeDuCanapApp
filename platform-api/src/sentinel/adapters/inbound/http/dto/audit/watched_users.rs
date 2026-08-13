use serde::Serialize;

use crate::sentinel::adapters::inbound::http::dto::audit::security::SecurityEventResponseDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::actions::ModerationActionResponseDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::infractions::InfractionResponseDto;
use crate::sentinel::adapters::inbound::http::dto::moderation::notes::UserNoteDto;
use platform_core::sentinel::domain::entities::audit::watched_user::WatchedUser;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::ports::inbound::audit::manage_watched_users::UserDossier;

#[derive(Debug, Serialize)]
pub struct WatchedUserResponseDto {
    pub user_id: UserId,
    pub username: String,
    pub guild_id: GuildId,
    pub guild_name: String,
    pub risk_level: String,
    pub total_warns: i64,
    pub total_mutes: i64,
    pub total_bans: i64,
    pub last_incident_at: Option<String>,
    pub security_events_count: i64,
    pub first_seen_at: String,
}

impl From<WatchedUser> for WatchedUserResponseDto {
    fn from(u: WatchedUser) -> Self {
        Self {
            user_id: u.user_id,
            username: u.username,
            guild_id: u.guild_id,
            guild_name: u.guild_name,
            risk_level: u.risk_level,
            total_warns: u.total_warns,
            total_mutes: u.total_mutes,
            total_bans: u.total_bans,
            last_incident_at: u.last_incident_at.map(|dt| dt.to_rfc3339()),
            security_events_count: u.security_events_count,
            first_seen_at: u.first_seen_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserDossierResponseDto {
    pub user: WatchedUserResponseDto,
    pub infractions: Vec<InfractionResponseDto>,
    pub moderation_actions: Vec<ModerationActionResponseDto>,
    pub security_events: Vec<SecurityEventResponseDto>,
    pub notes: Vec<UserNoteDto>,
}

impl From<UserDossier> for UserDossierResponseDto {
    fn from(d: UserDossier) -> Self {
        Self {
            user: WatchedUserResponseDto::from(d.user),
            infractions: d
                .infractions
                .into_iter()
                .map(InfractionResponseDto::from)
                .collect(),
            moderation_actions: d
                .moderation_actions
                .into_iter()
                .map(ModerationActionResponseDto::from)
                .collect(),
            security_events: d
                .security_events
                .into_iter()
                .map(SecurityEventResponseDto::from)
                .collect(),
            notes: d.notes.into_iter().map(UserNoteDto::from).collect(),
        }
    }
}

#[cfg(test)]
#[path = "tests/watched_users.rs"]
mod tests;
