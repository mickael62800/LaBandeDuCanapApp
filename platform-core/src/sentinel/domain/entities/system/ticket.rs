use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    /// Identifiant interne du ticket, distinct de l'identifiant du salon Discord.
    pub id: Uuid,
    /// Sujet affiche dans la liste et l'interface de support.
    pub title: String,
    /// Etat de workflow (`open`, `pending`, `closed`, etc.) partage par API,
    /// bot et worker.
    pub status: String,
    /// Niveau de priorite utilise par les files et le SLA.
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    /// Moderateur ou equipe actuellement responsable.
    pub assigned_to: Option<String>,
    pub server: String,
    /// Snowflake de la guild Discord proprietaire du ticket. `None` pour les
    /// lignes legacy d'avant la migration 296 (backfill non resolu) : l'acces
    /// web a ces tickets est refuse (fail-closed), seul le bot (gRPC) y accede.
    pub guild_id: Option<String>,
    pub category: String,
    pub ticket_type: String,
    /// Salon texte cree pour le ticket, absent avant sa creation Discord.
    pub channel_id: Option<String>,
    /// Salon vocal associe lorsqu'un accompagnement vocal est necessaire.
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketMessage {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub author_name: String,
    pub author_role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDetail {
    pub ticket: Ticket,
    pub messages: Vec<TicketMessage>,
}
