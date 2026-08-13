use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserStats {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub username: String,
    pub message_count: u64,
    pub voice_seconds: u64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildStatsOverview {
    pub guild_id: GuildId,
    pub total_messages: u64,
    pub total_voice_seconds: u64,
    pub active_members: u64,
    pub total_infractions: u64,
    pub total_warns: u64,
    pub total_mutes: u64,
    pub total_bans: u64,
    pub top_members: Vec<UserStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceSessionStats {
    pub channel_id: ChannelId,
    pub channel_name: String,
    pub is_temporary: bool,
    pub total_sessions: i64,
    pub total_duration_secs: i64,
    pub unique_users: i64,
    pub avg_duration_secs: i64,
    pub last_activity: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuildVoiceStats {
    pub total_channels: i64,
    pub total_sessions: i64,
    pub total_duration_secs: i64,
    pub unique_users: i64,
    pub avg_session_secs: i64,
    pub temp_channels: i64,
    pub perm_channels: i64,
    pub channels: Vec<VoiceSessionStats>,
}
