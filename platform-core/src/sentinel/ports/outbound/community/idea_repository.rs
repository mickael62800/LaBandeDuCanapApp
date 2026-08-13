use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::idea::{Idea, IdeaMessage};
use crate::sentinel::domain::errors::DomainError;

/// Filtres de listing des idees (tous optionnels, combines en AND).
#[derive(Debug, Default, Clone)]
pub struct IdeaFilters<'a> {
    pub guild_id: Option<&'a str>,
    pub status: Option<&'a str>,
    pub category: Option<&'a str>,
    pub author_id: Option<&'a str>,
    /// Recherche plein texte simple sur le titre et la description.
    pub search: Option<&'a str>,
}

#[async_trait]
pub trait IdeaRepository: Send + Sync {
    async fn find_all(
        &self,
        filters: IdeaFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Idea>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Idea>, DomainError>;
    async fn find_by_channel(&self, channel_id: &str) -> Result<Option<Idea>, DomainError>;
    async fn create(&self, idea: &Idea) -> Result<(), DomainError>;
    async fn update(&self, idea: &Idea) -> Result<(), DomainError>;
    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Nombre d'idees non tranchees (nouvelle / en discussion) d'un membre sur
    /// une guild. Sert au quota `max_open_per_user`.
    async fn count_open_by_author(
        &self,
        guild_id: &str,
        author_id: &str,
    ) -> Result<i64, DomainError>;

    async fn find_messages(&self, idea_id: Uuid) -> Result<Vec<IdeaMessage>, DomainError>;
    async fn save_message(&self, message: &IdeaMessage) -> Result<(), DomainError>;
}
