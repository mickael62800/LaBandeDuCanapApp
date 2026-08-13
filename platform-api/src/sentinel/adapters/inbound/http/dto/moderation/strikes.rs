use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use platform_core::sentinel::domain::entities::moderation::action::strikes::StrikeConfig;
use platform_core::sentinel::domain::entities::moderation::action::strikes::StrikeResult;
use platform_core::sentinel::domain::entities::moderation::action::strikes::StrikeThreshold;
use platform_core::sentinel::domain::entities::moderation::action::strikes::UserStrike;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use platform_core::sentinel::ports::inbound::moderation::manage_strikes::AddStrikeCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_strikes::SaveStrikeConfigCommand;
// ── Request DTOs ──

#[derive(Debug, Deserialize)]
pub struct AddStrikeDto {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    pub source: String,
    pub infraction_id: Option<String>,
}

impl From<AddStrikeDto> for AddStrikeCommand {
    fn from(dto: AddStrikeDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            user_id: dto.user_id,
            reason: dto.reason,
            source: dto.source,
            infraction_id: dto.infraction_id.and_then(|s| Uuid::parse_str(&s).ok()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SaveStrikeConfigDto {
    pub window_secs: i64,
    pub thresholds: Vec<StrikeThresholdDto>,
    pub enabled: bool,
}

impl SaveStrikeConfigDto {
    pub fn into_command(self, guild_id: GuildId) -> SaveStrikeConfigCommand {
        SaveStrikeConfigCommand {
            guild_id,
            window_secs: self.window_secs,
            thresholds: self
                .thresholds
                .into_iter()
                .map(StrikeThreshold::from)
                .collect(),
            enabled: self.enabled,
        }
    }
}

// ── Response DTOs ──

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StrikeThresholdDto {
    pub strikes: u32,
    pub action: String,
    pub duration: Option<u64>,
}

impl From<StrikeThreshold> for StrikeThresholdDto {
    fn from(t: StrikeThreshold) -> Self {
        Self {
            strikes: t.strikes,
            action: t.action,
            duration: t.duration,
        }
    }
}

impl From<StrikeThresholdDto> for StrikeThreshold {
    fn from(dto: StrikeThresholdDto) -> Self {
        Self {
            strikes: dto.strikes,
            action: dto.action,
            duration: dto.duration,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StrikeConfigDto {
    pub guild_id: GuildId,
    pub window_secs: i64,
    pub thresholds: Vec<StrikeThresholdDto>,
    pub enabled: bool,
}

impl From<StrikeConfig> for StrikeConfigDto {
    fn from(c: StrikeConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            window_secs: c.window_secs,
            thresholds: c
                .thresholds
                .into_iter()
                .map(StrikeThresholdDto::from)
                .collect(),
            enabled: c.enabled,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct UserStrikeDto {
    pub id: String,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    pub source: String,
    pub infraction_id: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

impl From<UserStrike> for UserStrikeDto {
    fn from(s: UserStrike) -> Self {
        Self {
            id: s.id.to_string(),
            guild_id: s.guild_id,
            user_id: s.user_id,
            reason: s.reason,
            source: s.source,
            infraction_id: s.infraction_id.map(|u| u.to_string()),
            expires_at: s.expires_at.map(|d| d.to_rfc3339()),
            created_at: s.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StrikeResultDto {
    pub id: String,
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    pub source: String,
    pub active_count: u32,
    pub escalation_action: Option<String>,
    pub escalation_duration: Option<u64>,
    pub created_at: String,
}

impl From<StrikeResult> for StrikeResultDto {
    fn from(r: StrikeResult) -> Self {
        Self {
            id: r.strike.id.to_string(),
            guild_id: r.strike.guild_id,
            user_id: r.strike.user_id,
            reason: r.strike.reason,
            source: r.strike.source,
            active_count: r.active_count,
            escalation_action: r.escalation_action,
            escalation_duration: r.escalation_duration,
            created_at: r.strike.created_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/strikes.rs"]
mod tests;
