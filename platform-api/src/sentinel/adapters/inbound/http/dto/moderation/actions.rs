use platform_core::sentinel::domain::entities::moderation::action::applied::ModerationAction;
use platform_core::sentinel::domain::entities::moderation::action::applied::UserModerationHistory;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct LogActionDto {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub gravity: Option<String>,
    pub duration: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ModerationActionResponseDto {
    pub id: String,
    pub action_type: String,
    pub target_name: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikes_count: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UserHistoryDto {
    pub target_id: String,
    pub target_name: String,
    pub total_warns: u32,
    pub total_mutes: u32,
    pub total_bans: u32,
    pub actions: Vec<ModerationActionResponseDto>,
}

/// MOD #7 — Entree d'agregation par moderateur sur une fenetre glissante.
#[derive(Debug, Serialize)]
pub struct ModStatsEntryDto {
    pub moderator_id: String,
    pub moderator_name: String,
    pub total: i64,
    pub warns: i64,
    pub mutes: i64,
    pub bans: i64,
    pub kicks: i64,
}

impl From<LogActionDto> for LogModerationCommand {
    fn from(dto: LogActionDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            channel_id: dto.channel_id,
            moderator_id: dto.moderator_id,
            moderator_name: dto.moderator_name,
            target_id: dto.target_id,
            target_name: dto.target_name,
            action_type: dto.action_type,
            reason: dto.reason,
            gravity: dto.gravity,
            duration: dto.duration,
        }
    }
}

impl From<ModerationAction> for ModerationActionResponseDto {
    fn from(a: ModerationAction) -> Self {
        Self {
            id: a.id.to_string(),
            action_type: a.action_type,
            target_name: a.target_name,
            reason: a.reason,
            escalation_action: None,
            escalation_duration: None,
            strikes_count: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BanEntryDto {
    pub id: String,
    pub guild_id: GuildId,
    pub target_id: String,
    pub target_name: String,
    pub moderator_name: String,
    pub action_type: String,
    pub reason: String,
    pub created_at: String,
}

impl From<ModerationAction> for BanEntryDto {
    fn from(a: ModerationAction) -> Self {
        Self {
            id: a.id.to_string(),
            guild_id: a.guild_id,
            target_id: a.target_id,
            target_name: a.target_name,
            moderator_name: a.moderator_name,
            action_type: a.action_type,
            reason: a.reason,
            created_at: a.created_at.to_rfc3339(),
        }
    }
}

impl From<UserModerationHistory> for UserHistoryDto {
    fn from(h: UserModerationHistory) -> Self {
        Self {
            target_id: h.target_id,
            target_name: h.target_name,
            total_warns: h.total_warns,
            total_mutes: h.total_mutes,
            total_bans: h.total_bans,
            actions: h
                .actions
                .into_iter()
                .map(ModerationActionResponseDto::from)
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
