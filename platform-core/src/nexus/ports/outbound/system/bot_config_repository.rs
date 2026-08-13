use async_trait::async_trait;

use crate::nexus::domain::entities::system::bot_config::BotDefinition;
use crate::nexus::domain::entities::system::bot_config::BotGuildConfig;
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait BotConfigRepository: Send + Sync {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError>;
    async fn get_config(
        &self,
        guild_id: &str,
        bot_name: &str,
    ) -> Result<Vec<BotGuildConfig>, DomainError>;
    async fn get_all_config(&self, guild_id: &str) -> Result<Vec<BotGuildConfig>, DomainError>;
    async fn set_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), DomainError>;
    async fn delete_config(
        &self,
        guild_id: &str,
        bot_name: &str,
        key: &str,
    ) -> Result<(), DomainError>;
}
