use crate::sentinel::domain::entities::system::discord_ids::ChannelId;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::RoleId;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RolePanel {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: Option<String>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub max_roles: Option<i32>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RolePanelEntry {
    pub id: Uuid,
    pub panel_id: Uuid,
    pub role_id: RoleId,
    pub role_name: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
    pub position: i32,
}

#[derive(Debug, Clone)]
pub struct RolePanelDetail {
    pub panel: RolePanel,
    pub entries: Vec<RolePanelEntry>,
}

#[derive(Debug, Clone)]
pub struct AutoRole {
    pub id: Uuid,
    pub guild_id: GuildId,
    pub role_id: RoleId,
    pub role_name: String,
    pub delay_secs: i32,
    pub enabled: bool,
}
