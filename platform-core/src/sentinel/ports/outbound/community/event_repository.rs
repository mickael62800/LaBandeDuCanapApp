use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::sentinel::domain::entities::community::event::{
    CommunityEvent, EventAnswer, EventParticipant, UpsertEventCommand,
};
use crate::sentinel::domain::errors::DomainError;

/// Fenetre d'affichage du calendrier. Une vue semaine et une vue mois ne
/// different que par ses bornes.
#[derive(Debug, Clone, Copy)]
pub struct EventWindow {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

#[async_trait]
pub trait EventRepository: Send + Sync {
    /// Evenements CHEVAUCHANT la fenetre — pas seulement ceux qui y
    /// commencent. Sans quoi une campagne de trois semaines disparaitrait de
    /// toutes les semaines sauf la premiere.
    ///
    /// `public_only` sert la page publique : elle ne doit voir ni les
    /// brouillons, ni les evenements reserves aux membres.
    async fn list_in_window(
        &self,
        guild_id: &str,
        window: EventWindow,
        public_only: bool,
    ) -> Result<Vec<CommunityEvent>, DomainError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<CommunityEvent>, DomainError>;

    async fn create(&self, cmd: &UpsertEventCommand) -> Result<CommunityEvent, DomainError>;

    async fn update(
        &self,
        id: Uuid,
        cmd: &UpsertEventCommand,
    ) -> Result<Option<CommunityEvent>, DomainError>;

    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;

    // ── Inscriptions ──

    async fn list_participants(&self, event_id: Uuid)
        -> Result<Vec<EventParticipant>, DomainError>;

    /// Idempotent : reinscrire quelqu'un met simplement sa reponse a jour.
    async fn set_participation(
        &self,
        event_id: Uuid,
        user_id: &str,
        username: &str,
        answer: EventAnswer,
    ) -> Result<(), DomainError>;

    async fn remove_participation(
        &self,
        event_id: Uuid,
        user_id: &str,
    ) -> Result<bool, DomainError>;
}
