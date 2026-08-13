use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::news::{NewsPost, UpsertNewsCommand};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait NewsRepository: Send + Sync {
    /// Nouvelles d'une guilde : epinglees d'abord, puis par date decroissante.
    ///
    /// `published_only` sert la page publique : elle ne doit voir ni les
    /// nouvelles reservees aux membres, ni celles datees du futur.
    async fn list(
        &self,
        guild_id: &str,
        published_only: bool,
        limit: i64,
    ) -> Result<Vec<NewsPost>, DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<NewsPost>, DomainError>;

    async fn create(&self, cmd: &UpsertNewsCommand) -> Result<NewsPost, DomainError>;

    async fn update(
        &self,
        id: Uuid,
        cmd: &UpsertNewsCommand,
    ) -> Result<Option<NewsPost>, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;
}
