use async_trait::async_trait;

use crate::sentinel::domain::entities::system::ticket::Ticket;
use crate::sentinel::domain::entities::system::ticket::TicketDetail;
use crate::sentinel::domain::errors::DomainError;

pub struct CreateTicketCommand {
    pub title: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    /// Snowflake de la guild Discord (passe par le bot via gRPC, ou par le web).
    pub guild_id: Option<String>,
    pub category: String,
    pub ticket_type: String,
    pub channel_id: Option<String>,
}

pub struct UpdateTicketChannelCommand {
    pub ticket_id: String,
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
}

pub struct ReplyTicketCommand {
    pub ticket_id: String,
    pub content: String,
    pub author_name: String,
    pub author_role: String,
}

pub struct AssignTicketCommand {
    pub ticket_id: String,
    pub assignee: String,
}

#[async_trait]
pub trait ManageTicketsUseCase: Send + Sync {
    async fn list_tickets(
        &self,
        status: Option<String>,
        priority: Option<String>,
        search: Option<String>,
        author_id: Option<String>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Ticket>, DomainError>;
    async fn get_ticket_detail(&self, id: &str) -> Result<TicketDetail, DomainError>;
    async fn create_ticket(&self, command: CreateTicketCommand) -> Result<Ticket, DomainError>;
    async fn reply_ticket(&self, command: ReplyTicketCommand) -> Result<(), DomainError>;
    /// Ferme un ticket. Renvoie `true` si CE close a fait la transition (sinon
    /// deja ferme) -> permet a l'appelant de n'envoyer transcript/DM/delete
    /// qu'une fois (anti double-fermeture).
    async fn close_ticket(&self, id: &str) -> Result<bool, DomainError>;
    async fn assign_ticket(&self, command: AssignTicketCommand) -> Result<(), DomainError>;
    async fn update_status(&self, id: &str, status: &str) -> Result<(), DomainError>;
    async fn update_ticket_channel(
        &self,
        command: UpdateTicketChannelCommand,
    ) -> Result<(), DomainError>;
    async fn update_priority(&self, id: uuid::Uuid, priority: &str) -> Result<(), DomainError>;
    async fn update_sla(
        &self,
        id: uuid::Uuid,
        first_response_at: Option<&str>,
        resolved_at: Option<&str>,
        satisfaction_rating: Option<i32>,
    ) -> Result<(), DomainError>;

    // `moderated_guilds` a ete retire avec le RBAC multi-roles : il lisait
    // `api_user_guilds`, supprimee par la migration 007. Le back-office est
    // superadmin-only, `list_tickets` n'a plus de scope par role a appliquer.

    /// Suppression en masse des tickets selon des filtres optionnels
    /// (`author_id`, plage `created_at`). Renvoie le nombre de tickets supprimes.
    async fn bulk_delete_tickets(
        &self,
        author_id: Option<&str>,
        from: Option<chrono::DateTime<chrono::Utc>>,
        to: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<u64, DomainError>;
}
