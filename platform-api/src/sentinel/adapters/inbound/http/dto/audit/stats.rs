use platform_core::sentinel::domain::entities::audit::user_stats::GuildStatsOverview;
use platform_core::sentinel::domain::entities::audit::user_stats::GuildVoiceStats;
use platform_core::sentinel::domain::entities::audit::user_stats::UserStats;
use platform_core::sentinel::domain::entities::audit::user_stats::VoiceSessionStats;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use platform_core::sentinel::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use serde::Deserialize;
use serde::Serialize;
// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct RecordMessagesDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub count: u64,
}

#[derive(Debug, Deserialize)]
pub struct RecordVoiceDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub seconds: u64,
    #[serde(default)]
    pub channel_id: Option<ChannelId>,
    #[serde(default)]
    pub channel_name: String,
}

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    pub limit: Option<u32>,
}

// ── Response DTOs ──

#[derive(Debug, Serialize)]
pub struct UserStatsDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
    pub voice_hours: f64,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct GuildOverviewDto {
    pub guild_id: GuildId,
    pub total_messages: u64,
    pub total_voice_seconds: u64,
    pub total_voice_hours: f64,
    pub active_members: u64,
    pub total_infractions: u64,
    pub total_warns: u64,
    pub total_mutes: u64,
    pub total_bans: u64,
    pub top_members: Vec<UserStatsDto>,
}

// ── Voice Stats ──

#[derive(Debug, Deserialize)]
pub struct VoiceStatsQuery {
    pub days: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct VoiceSessionStatsDto {
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub is_temporary: bool,
    pub total_sessions: i64,
    pub total_duration_secs: i64,
    pub total_duration_hours: f64,
    pub unique_users: i64,
    pub avg_duration_secs: i64,
    pub last_activity: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuildVoiceStatsDto {
    pub total_channels: i64,
    pub total_sessions: i64,
    pub total_duration_secs: i64,
    pub total_duration_hours: f64,
    pub unique_users: i64,
    pub avg_session_secs: i64,
    pub temp_channels: i64,
    pub perm_channels: i64,
    pub channels: Vec<VoiceSessionStatsDto>,
}

impl From<VoiceSessionStats> for VoiceSessionStatsDto {
    fn from(s: VoiceSessionStats) -> Self {
        Self {
            channel_id: s.channel_id,
            channel_name: s.channel_name,
            is_temporary: s.is_temporary,
            total_sessions: s.total_sessions,
            total_duration_secs: s.total_duration_secs,
            total_duration_hours: s.total_duration_secs as f64 / 3600.0,
            unique_users: s.unique_users,
            avg_duration_secs: s.avg_duration_secs,
            last_activity: s.last_activity.map(|t| t.to_rfc3339()),
        }
    }
}

impl From<GuildVoiceStats> for GuildVoiceStatsDto {
    fn from(g: GuildVoiceStats) -> Self {
        Self {
            total_channels: g.total_channels,
            total_sessions: g.total_sessions,
            total_duration_secs: g.total_duration_secs,
            total_duration_hours: g.total_duration_secs as f64 / 3600.0,
            unique_users: g.unique_users,
            avg_session_secs: g.avg_session_secs,
            temp_channels: g.temp_channels,
            perm_channels: g.perm_channels,
            channels: g
                .channels
                .into_iter()
                .map(VoiceSessionStatsDto::from)
                .collect(),
        }
    }
}

// ── Conversions ──

impl From<RecordMessagesDto> for RecordMessagesCommand {
    fn from(dto: RecordMessagesDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            count: dto.count,
        }
    }
}

impl From<RecordVoiceDto> for RecordVoiceCommand {
    fn from(dto: RecordVoiceDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            username: dto.username,
            seconds: dto.seconds,
            channel_id: dto.channel_id.unwrap_or_else(|| String::new().into()),
            channel_name: dto.channel_name,
        }
    }
}

impl From<UserStats> for UserStatsDto {
    fn from(s: UserStats) -> Self {
        Self {
            guild_id: s.guild_id,
            user_id: s.user_id,
            username: s.username,
            voice_hours: s.voice_seconds as f64 / 3600.0,
            message_count: s.message_count,
            voice_seconds: s.voice_seconds,
            updated_at: s.updated_at.to_rfc3339(),
        }
    }
}

impl From<GuildStatsOverview> for GuildOverviewDto {
    fn from(o: GuildStatsOverview) -> Self {
        Self {
            guild_id: o.guild_id,
            total_messages: o.total_messages,
            total_voice_seconds: o.total_voice_seconds,
            total_voice_hours: o.total_voice_seconds as f64 / 3600.0,
            active_members: o.active_members,
            total_infractions: o.total_infractions,
            total_warns: o.total_warns,
            total_mutes: o.total_mutes,
            total_bans: o.total_bans,
            top_members: o.top_members.into_iter().map(UserStatsDto::from).collect(),
        }
    }
}

#[cfg(test)]
#[path = "tests/stats.rs"]
mod tests;
