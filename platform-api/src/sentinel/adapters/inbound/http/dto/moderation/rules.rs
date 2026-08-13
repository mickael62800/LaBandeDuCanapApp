use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use platform_core::sentinel::domain::entities::system::rule::Rule;
use platform_core::sentinel::domain::enums::moderation::flag_type::FlagType;
use platform_core::sentinel::ports::inbound::moderation::manage_rules::CreateRuleCommand;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct CreateRuleDto {
    pub guild_id: GuildId,
    pub flag_type: String,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct RuleResponseDto {
    pub id: String,
    pub guild_id: GuildId,
    pub flag_type: String,
    pub weight: f64,
    pub threshold_warn: f64,
    pub threshold_delete: f64,
    pub threshold_mute: f64,
    pub threshold_ban: f64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<CreateRuleDto> for CreateRuleCommand {
    fn from(dto: CreateRuleDto) -> Self {
        Self {
            guild_id: dto.guild_id,
            flag_type: FlagType::from_str_lossy(&dto.flag_type),
            weight: dto.weight,
            threshold_warn: dto.threshold_warn,
            threshold_delete: dto.threshold_delete,
            threshold_mute: dto.threshold_mute,
            threshold_ban: dto.threshold_ban,
            enabled: dto.enabled,
        }
    }
}

impl From<Rule> for RuleResponseDto {
    fn from(rule: Rule) -> Self {
        Self {
            id: rule.id.to_string(),
            guild_id: rule.guild_id,
            flag_type: rule.flag_type.as_str().to_string(),
            weight: rule.weight,
            threshold_warn: rule.threshold_warn,
            threshold_delete: rule.threshold_delete,
            threshold_mute: rule.threshold_mute,
            threshold_ban: rule.threshold_ban,
            enabled: rule.enabled,
            created_at: rule.created_at.to_rfc3339(),
            updated_at: rule.updated_at.to_rfc3339(),
        }
    }
}

#[cfg(test)]
#[path = "tests/rules.rs"]
mod tests;
