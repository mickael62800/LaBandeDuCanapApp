use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::idea::{Idea, IdeaDetail, IdeaMessage};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::community::idea_repository::IdeaFilters;

/// Proposition d'une nouvelle idee (depuis la modale Discord ou le web).
pub struct CreateIdeaCommand {
    pub guild_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub author_id: String,
    pub author_name: String,
    /// Salon prive deja cree par le bot, s'il l'a cree avant d'enregistrer.
    pub channel_id: Option<String>,
}

/// Decision du staff sur une idee.
pub struct DecideIdeaCommand {
    pub id: Uuid,
    pub status: String,
    pub decided_by: String,
    pub decided_by_name: String,
    pub reason: Option<String>,
}

pub struct AddIdeaMessageCommand {
    pub idea_id: Uuid,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
}

#[async_trait]
pub trait ManageIdeasUseCase: Send + Sync {
    async fn list(
        &self,
        filters: IdeaFilters<'_>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Idea>, DomainError>;
    async fn get(&self, id: Uuid) -> Result<Idea, DomainError>;
    async fn get_detail(&self, id: Uuid) -> Result<IdeaDetail, DomainError>;
    async fn get_by_channel(&self, channel_id: &str) -> Result<Option<Idea>, DomainError>;

    async fn create(&self, cmd: CreateIdeaCommand) -> Result<Idea, DomainError>;

    /// Change le statut en respectant la machine a etats `IdeaStatus`.
    /// Enregistre l'auteur de la decision et son motif.
    async fn decide(&self, cmd: DecideIdeaCommand) -> Result<Idea, DomainError>;

    /// Rattache (ou detache) le salon Discord dedie a l'idee.
    async fn set_channel(&self, id: Uuid, channel_id: Option<&str>) -> Result<Idea, DomainError>;

    async fn add_message(&self, cmd: AddIdeaMessageCommand) -> Result<IdeaMessage, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Nombre d'idees non tranchees d'un membre (quota d'ouverture).
    async fn count_open_by_author(
        &self,
        guild_id: &str,
        author_id: &str,
    ) -> Result<i64, DomainError>;
}
