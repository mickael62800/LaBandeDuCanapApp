use platform_core::sentinel::domain::entities::system::bot_config::BotDefinition;
use platform_core::sentinel::domain::entities::system::bot_config::BotGuildConfig;
use platform_core::sentinel::domain::entities::system::discord_ids::GuildId;
use serde::Deserialize;
use serde::Serialize;
#[derive(Debug, Serialize, Deserialize)]
pub struct BotDefinitionDto {
    pub bot_name: String,
    pub display_name: String,
    pub description: String,
    pub config_schema: serde_json::Value,
}

impl From<BotDefinition> for BotDefinitionDto {
    fn from(d: BotDefinition) -> Self {
        Self {
            bot_name: d.bot_name,
            display_name: d.display_name,
            description: d.description,
            config_schema: d.config_schema,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotGuildConfigDto {
    pub guild_id: GuildId,
    pub bot_name: String,
    pub config_key: String,
    pub config_value: String,
}

impl From<BotGuildConfig> for BotGuildConfigDto {
    fn from(c: BotGuildConfig) -> Self {
        Self {
            guild_id: c.guild_id,
            bot_name: c.bot_name,
            config_key: c.config_key,
            config_value: c.config_value,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SetConfigDto {
    pub guild_id: GuildId,
    pub bot_name: String,
    pub config_key: String,
    pub config_value: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteConfigDto {
    pub guild_id: GuildId,
    pub bot_name: String,
    pub config_key: String,
}

#[cfg(test)]
#[path = "tests/bot_config.rs"]
mod tests;
