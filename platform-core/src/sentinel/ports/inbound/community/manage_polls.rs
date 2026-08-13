use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::poll::{Poll, UpsertPollCommand};
use crate::sentinel::domain::errors::DomainError;

/// Un sondage accompagne du choix de celui qui le regarde, pour pre-cocher
/// son vote. `None` pour un visiteur non connecte.
#[derive(Debug, Clone)]
pub struct PollWithVote {
    pub poll: Poll,
    pub my_vote: Option<Uuid>,
}

#[async_trait]
pub trait ManagePollsUseCase: Send + Sync {
    async fn list(
        &self,
        guild_id: &str,
        open_only: bool,
        limit: i64,
    ) -> Result<Vec<Poll>, DomainError>;

    async fn get(&self, id: Uuid, viewer_id: Option<&str>) -> Result<PollWithVote, DomainError>;

    async fn create(&self, cmd: UpsertPollCommand) -> Result<Poll, DomainError>;

    async fn close(&self, id: Uuid) -> Result<(), DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    /// Voter. Remplace le vote precedent le cas echeant.
    async fn vote(
        &self,
        poll_id: Uuid,
        option_id: Uuid,
        user_id: &str,
    ) -> Result<Poll, DomainError>;
}
