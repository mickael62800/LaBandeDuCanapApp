use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::spotlight::{Spotlight, UpsertSpotlightCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageSpotlightUseCase: Send + Sync {
    /// Celui a mettre en avant : la periode demandee si elle existe, sinon le
    /// plus recent.
    async fn current(
        &self,
        guild_id: &str,
        period: Option<&str>,
    ) -> Result<Option<Spotlight>, DomainError>;

    async fn list(&self, guild_id: &str, limit: i64) -> Result<Vec<Spotlight>, DomainError>;

    async fn designate(&self, cmd: UpsertSpotlightCommand) -> Result<Spotlight, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
