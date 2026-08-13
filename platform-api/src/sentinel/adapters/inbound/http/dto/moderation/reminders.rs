use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::action::sanction_reminder::SanctionReminder;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::ports::inbound::moderation::manage_reminders::CreateReminderCommand;

#[derive(Debug, Deserialize)]
pub struct CreateReminderDto {
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub action_id: String,
    pub duration_secs: u64,
    #[serde(default = "default_remind_before")]
    pub remind_before_secs: u64,
}

fn default_remind_before() -> u64 {
    3600
}

impl From<CreateReminderDto> for CreateReminderCommand {
    fn from(dto: CreateReminderDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            moderator_id: dto.moderator_id,
            moderator_name: dto.moderator_name,
            target_id: dto.target_id,
            target_name: dto.target_name,
            action_type: dto.action_type,
            reason: dto.reason,
            action_id: Uuid::parse_str(&dto.action_id).unwrap_or_else(|e| {
                tracing::warn!(error = %e, action_id = %dto.action_id, "UUID action_id invalide dans reminder, utilisation UUID nil");
                Uuid::nil()
            }),
            duration_secs: dto.duration_secs,
            remind_before_secs: dto.remind_before_secs,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SanctionReminderDto {
    pub id: String,
    pub guild_id: GuildId,
    pub moderator_id: String,
    pub moderator_name: String,
    pub target_id: String,
    pub target_name: String,
    pub action_type: String,
    pub reason: String,
    pub action_id: String,
    pub remind_at: String,
    pub expires_at: String,
    pub status: String,
    pub created_at: String,
}

impl From<SanctionReminder> for SanctionReminderDto {
    fn from(r: SanctionReminder) -> Self {
        Self {
            id: r.id.to_string(),
            guild_id: r.guild_id,
            moderator_id: r.moderator_id,
            moderator_name: r.moderator_name,
            target_id: r.target_id,
            target_name: r.target_name,
            action_type: r.action_type,
            reason: r.reason,
            action_id: r.action_id.to_string(),
            remind_at: r.remind_at.to_rfc3339(),
            expires_at: r.expires_at.to_rfc3339(),
            status: r.status,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/reminders.rs"]
mod tests;
