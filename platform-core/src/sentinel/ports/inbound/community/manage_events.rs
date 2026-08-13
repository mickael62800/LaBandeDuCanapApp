use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::community::event::{
    CommunityEvent, EventAnswer, EventParticipant, UpsertEventCommand,
};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::community::event_repository::EventWindow;

/// Un evenement accompagne de ses inscrits — ce que le calendrier affiche.
#[derive(Debug, Clone)]
pub struct EventWithParticipants {
    pub event: CommunityEvent,
    pub participants: Vec<EventParticipant>,
}

#[async_trait]
pub trait ManageEventsUseCase: Send + Sync {
    async fn list_window(
        &self,
        guild_id: &str,
        window: EventWindow,
        public_only: bool,
    ) -> Result<Vec<CommunityEvent>, DomainError>;

    async fn get(&self, id: Uuid) -> Result<EventWithParticipants, DomainError>;

    async fn create(&self, cmd: UpsertEventCommand) -> Result<CommunityEvent, DomainError>;

    async fn update(
        &self,
        id: Uuid,
        cmd: UpsertEventCommand,
    ) -> Result<CommunityEvent, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<(), DomainError>;

    async fn join(
        &self,
        event_id: Uuid,
        user_id: &str,
        username: &str,
        answer: EventAnswer,
    ) -> Result<(), DomainError>;

    async fn leave(&self, event_id: Uuid, user_id: &str) -> Result<(), DomainError>;
}
