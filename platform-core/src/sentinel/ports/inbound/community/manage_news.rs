use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::news::{NewsPost, UpsertNewsCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageNewsUseCase: Send + Sync {
    async fn list(
        &self,
        guild_id: &str,
        published_only: bool,
        limit: i64,
    ) -> Result<Vec<NewsPost>, DomainError>;

    async fn get(&self, id: Uuid) -> Result<NewsPost, DomainError>;

    async fn create(&self, cmd: UpsertNewsCommand) -> Result<NewsPost, DomainError>;

    async fn update(&self, id: Uuid, cmd: UpsertNewsCommand) -> Result<NewsPost, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;
}
