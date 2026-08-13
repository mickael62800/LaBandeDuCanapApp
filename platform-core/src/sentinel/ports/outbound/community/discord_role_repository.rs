use async_trait::async_trait;

use crate::sentinel::domain::entities::system::discord_role::DiscordRole;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait DiscordRoleRepository: Send + Sync {
    async fn sync_roles(&self, guild_id: &str, roles: Vec<DiscordRole>) -> Result<(), DomainError>;
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<DiscordRole>, DomainError>;
    async fn find_by_id(
        &self,
        guild_id: &str,
        role_id: &str,
    ) -> Result<Option<DiscordRole>, DomainError>;
}
