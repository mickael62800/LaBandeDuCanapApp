use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::ticket::Ticket;
use crate::sentinel::domain::entities::system::ticket::TicketMessage;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait TicketRepository: Send + Sync {
    async fn find_all(
        &self,
        status: Option<&str>,
        priority: Option<&str>,
        search: Option<&str>,
        author_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Ticket>, DomainError>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Ticket>, DomainError>;
    async fn save(&self, ticket: &Ticket) -> Result<(), DomainError>;
    async fn update_status(&self, id: Uuid, status: &str) -> Result<(), DomainError>;

    /// Ferme un ticket de facon ATOMIQUE : `WHERE status <> 'closed'`. Renvoie
    /// `true` uniquement si CE close a fait la transition -> l'appelant n'envoie
    /// le transcript / DM de satisfaction et ne supprime le salon qu'une fois.
    async fn close_if_open(&self, id: Uuid) -> Result<bool, DomainError>;
    async fn update_assignee(&self, id: Uuid, assignee: &str) -> Result<(), DomainError>;
    async fn find_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>, DomainError>;
    async fn save_message(&self, message: &TicketMessage) -> Result<(), DomainError>;
    async fn update_voice_channel(
        &self,
        id: Uuid,
        voice_channel_id: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn update_invited_user(
        &self,
        id: Uuid,
        invited_user_id: Option<&str>,
    ) -> Result<(), DomainError>;
    async fn update_priority(&self, id: Uuid, priority: &str) -> Result<(), DomainError>;
    async fn update_sla(
        &self,
        id: Uuid,
        first_response_at: Option<&str>,
        resolved_at: Option<&str>,
        satisfaction_rating: Option<i32>,
    ) -> Result<(), DomainError>;

    // `find_user_guild_roles` a ete retire : il faisait un SELECT sur
    // `api_user_guilds`, table supprimee par la migration 007. Toute execution
    // de cette methode partait donc en erreur SQL.

    /// Suppression en masse avec filtres optionnels combinables (AND) :
    /// `author_id`, borne basse et borne haute de `created_at`. Renvoie le
    /// nombre de tickets supprimes.
    async fn bulk_delete(
        &self,
        author_id: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError>;
}
