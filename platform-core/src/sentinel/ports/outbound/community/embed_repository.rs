use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::embed::Embed;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait EmbedRepository: Send + Sync {
    async fn create(&self, e: &Embed) -> Result<(), DomainError>;
    async fn update(&self, e: &Embed) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<Embed>, DomainError>;
    async fn list_by_guild(&self, guild_id: &str) -> Result<Vec<Embed>, DomainError>;
    /// Memorise ou l'embed vient d'etre poste (pour l'edition ulterieure).
    async fn set_last_post(
        &self,
        id: Uuid,
        channel_id: &str,
        message_id: &str,
    ) -> Result<(), DomainError>;
}
