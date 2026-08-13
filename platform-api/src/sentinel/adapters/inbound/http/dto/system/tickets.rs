use platform_core::sentinel::domain::entities::system::ticket::Ticket;
use platform_core::sentinel::domain::entities::system::ticket::TicketDetail;
use platform_core::sentinel::domain::entities::system::ticket::TicketMessage;
use platform_core::sentinel::ports::inbound::system::manage_tickets::CreateTicketCommand;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Default)]
pub struct ListTicketsQuery {
    pub status: Option<String>,
    pub priority: Option<String>,
    pub search: Option<String>,
    pub author_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTicketDto {
    pub title: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    /// Snowflake de la guild. Optionnel pour compat ascendante, mais REQUIS
    /// pour les appels web (gate Moderator+ scoped par guild cote handler).
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_ticket_type")]
    pub ticket_type: String,
    pub channel_id: Option<String>,
}

fn default_priority() -> String {
    "medium".to_string()
}

fn default_ticket_type() -> String {
    "autre".to_string()
}

#[derive(Debug, Deserialize)]
pub struct ReplyDto {
    pub content: String,
    #[serde(default = "default_author_name")]
    pub author_name: String,
    #[serde(default = "default_author_role")]
    pub author_role: String,
}

fn default_author_name() -> String {
    "staff".to_string()
}

fn default_author_role() -> String {
    "moderator".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AssignDto {
    pub assignee: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStatusDto {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTicketChannelDto {
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TicketResponseDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub assigned_to: Option<String>,
    pub server: String,
    pub guild_id: Option<String>,
    pub category: String,
    pub ticket_type: String,
    pub channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
}

#[derive(Debug, Serialize)]
pub struct TicketMessageDto {
    pub id: String,
    pub ticket_id: String,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct TicketDetailDto {
    pub ticket: TicketResponseDto,
    pub messages: Vec<TicketMessageDto>,
}

impl From<CreateTicketDto> for CreateTicketCommand {
    fn from(dto: CreateTicketDto) -> Self {
        Self {
            title: dto.title,
            priority: dto.priority,
            author_id: dto.author_id,
            author_name: dto.author_name,
            server: dto.server,
            guild_id: dto.guild_id,
            category: dto.category,
            ticket_type: dto.ticket_type,
            channel_id: dto.channel_id,
        }
    }
}

impl From<Ticket> for TicketResponseDto {
    fn from(t: Ticket) -> Self {
        Self {
            id: t.id.to_string(),
            title: t.title,
            status: t.status,
            priority: t.priority,
            author_id: t.author_id,
            author_name: t.author_name,
            assigned_to: t.assigned_to,
            server: t.server,
            guild_id: t.guild_id,
            category: t.category,
            ticket_type: t.ticket_type,
            channel_id: t.channel_id,
            voice_channel_id: t.voice_channel_id,
            invited_user_id: t.invited_user_id,
            created_at: t.created_at.to_rfc3339(),
            updated_at: t.updated_at.to_rfc3339(),
            messages_count: t.messages_count,
        }
    }
}

impl From<TicketMessage> for TicketMessageDto {
    fn from(m: TicketMessage) -> Self {
        Self {
            id: m.id.to_string(),
            ticket_id: m.ticket_id.to_string(),
            author_name: m.author_name,
            author_role: m.author_role,
            content: m.content,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

impl From<TicketDetail> for TicketDetailDto {
    fn from(d: TicketDetail) -> Self {
        Self {
            ticket: TicketResponseDto::from(d.ticket),
            messages: d.messages.into_iter().map(TicketMessageDto::from).collect(),
        }
    }
}

#[cfg(test)]
#[path = "tests/tickets.rs"]
mod tests;
