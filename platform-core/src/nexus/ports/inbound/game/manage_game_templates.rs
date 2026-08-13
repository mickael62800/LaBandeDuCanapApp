use async_trait::async_trait;
use uuid::Uuid;

use crate::nexus::domain::entities::game::template::GameTemplate;
use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait ManageGameTemplatesUseCase: Send + Sync {
    /// Liste tous les templates non-deleted, filtree par allowed_templates
    /// configure pour la guild (whitelist).
    async fn list_for_guild(&self, guild_id: &str) -> Result<Vec<GameTemplate>, DomainError>;
    async fn get(&self, id: Uuid) -> Result<GameTemplate, DomainError>;
    async fn get_by_slug(&self, slug: &str) -> Result<GameTemplate, DomainError>;
}
