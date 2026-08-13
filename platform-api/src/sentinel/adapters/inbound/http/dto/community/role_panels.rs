use platform_core::sentinel::domain::entities::community::role_panel::AutoRole;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanel;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelDetail;
use platform_core::sentinel::domain::entities::community::role_panel::RolePanelEntry;
use platform_core::sentinel::domain::entities::system::discord_ids::ChannelId;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::MessageId;
use platform_core::sentinel::domain::entities::system::discord_ids::RoleId;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::CreateAutoRoleCommand;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::CreateRolePanelCommand;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::CreateRolePanelEntryCommand;
use platform_core::sentinel::ports::inbound::community::manage_role_panels::SetMessageIdCommand;
use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Deserialize)]
pub struct CreateRolePanelDto {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_mode")]
    pub mode: String,
    pub max_roles: Option<i32>,
    pub entries: Vec<CreateEntryDto>,
}

fn default_mode() -> String {
    "button".to_string()
}

#[derive(Debug, Deserialize)]
pub struct CreateEntryDto {
    pub role_id: RoleId,
    pub role_name: String,
    pub emoji: Option<String>,
    #[serde(default)]
    pub label: String,
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default)]
    pub position: i32,
}

fn default_style() -> String {
    "primary".to_string()
}

#[derive(Debug, Deserialize)]
pub struct SetMessageIdDto {
    pub panel_id: String,
    pub message_id: MessageId,
}

#[derive(Debug, Deserialize)]
pub struct CreateAutoRoleDto {
    pub guild_id: GuildId,
    pub role_id: RoleId,
    pub role_name: String,
    #[serde(default)]
    pub delay_secs: i32,
}

#[derive(Debug, Serialize)]
pub struct RolePanelDto {
    pub id: String,
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub message_id: Option<String>,
    pub title: String,
    pub description: String,
    pub mode: String,
    pub max_roles: Option<i32>,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct RolePanelEntryDto {
    pub id: String,
    pub role_id: RoleId,
    pub role_name: String,
    pub emoji: Option<String>,
    pub label: String,
    pub style: String,
    pub position: i32,
}

#[derive(Debug, Serialize)]
pub struct RolePanelDetailDto {
    pub panel: RolePanelDto,
    pub entries: Vec<RolePanelEntryDto>,
}

#[derive(Debug, Serialize)]
pub struct AutoRoleDto {
    pub id: String,
    pub guild_id: GuildId,
    pub role_id: RoleId,
    pub role_name: String,
    pub delay_secs: i32,
    pub enabled: bool,
}

impl From<CreateRolePanelDto> for CreateRolePanelCommand {
    fn from(d: CreateRolePanelDto) -> Self {
        Self {
            guild_id: d.guild_id,
            channel_id: d.channel_id,
            title: d.title,
            description: d.description,
            mode: d.mode,
            max_roles: d.max_roles,
            entries: d
                .entries
                .into_iter()
                .map(|e| CreateRolePanelEntryCommand {
                    role_id: e.role_id,
                    role_name: e.role_name,
                    emoji: e.emoji,
                    label: e.label,
                    style: e.style,
                    position: e.position,
                })
                .collect(),
        }
    }
}

impl From<SetMessageIdDto> for SetMessageIdCommand {
    fn from(d: SetMessageIdDto) -> Self {
        Self {
            panel_id: d.panel_id,
            message_id: d.message_id,
        }
    }
}

impl From<CreateAutoRoleDto> for CreateAutoRoleCommand {
    fn from(d: CreateAutoRoleDto) -> Self {
        Self {
            guild_id: d.guild_id,
            role_id: d.role_id,
            role_name: d.role_name,
            delay_secs: d.delay_secs,
        }
    }
}

impl From<RolePanel> for RolePanelDto {
    fn from(p: RolePanel) -> Self {
        Self {
            id: p.id.to_string(),
            guild_id: p.guild_id,
            channel_id: p.channel_id,
            message_id: p.message_id,
            title: p.title,
            description: p.description,
            mode: p.mode,
            max_roles: p.max_roles,
            enabled: p.enabled,
            created_at: p.created_at.to_rfc3339(),
        }
    }
}

impl From<RolePanelEntry> for RolePanelEntryDto {
    fn from(e: RolePanelEntry) -> Self {
        Self {
            id: e.id.to_string(),
            role_id: e.role_id,
            role_name: e.role_name,
            emoji: e.emoji,
            label: e.label,
            style: e.style,
            position: e.position,
        }
    }
}

impl From<RolePanelDetail> for RolePanelDetailDto {
    fn from(d: RolePanelDetail) -> Self {
        Self {
            panel: d.panel.into(),
            entries: d.entries.into_iter().map(RolePanelEntryDto::from).collect(),
        }
    }
}

impl From<AutoRole> for AutoRoleDto {
    fn from(a: AutoRole) -> Self {
        Self {
            id: a.id.to_string(),
            guild_id: a.guild_id,
            role_id: a.role_id,
            role_name: a.role_name,
            delay_secs: a.delay_secs,
            enabled: a.enabled,
        }
    }
}

#[cfg(test)]
#[path = "tests/role_panels.rs"]
mod tests;
