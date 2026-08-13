//! Client API du module tickets (migre depuis ticket-bot).
//!
//! Phase 7A — Migration gRPC :
//! - Tout le domaine `tickets` (list, get, create, reply, close, status, assign,
//!   channels) passe par gRPC via `SentinelGrpcClient::tickets()`.

use std::sync::Arc;

use crate::shared::grpc_client::SentinelGrpcClient;
use serde::{Deserialize, Serialize};

use platform_proto::sentinel::tickets::v1 as proto;

// ── DTOs publics ──

#[derive(Debug, Deserialize)]
/// Miroir de la reponse ticket de l'API. Le bot n'en consomme qu'une partie
/// (id, statut, salon) ; les autres champs decrivent le contrat de l'API et
/// sont lus par le dashboard web, pas par le bot.
#[allow(dead_code)]
pub struct Ticket {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub assigned_to: Option<String>,
    pub server: String,
    pub category: String,
    pub ticket_type: Option<String>,
    pub channel_id: Option<String>,
    pub voice_channel_id: Option<String>,
    pub invited_user_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub messages_count: u32,
    #[serde(default)]
    pub first_response_at: Option<String>,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub satisfaction_rating: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct TicketMessage {
    pub author_name: String,
    pub author_role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct TicketDetail {
    pub ticket: Ticket,
    pub messages: Vec<TicketMessage>,
}

#[derive(Debug, Serialize)]
pub struct CreateTicketRequest {
    pub title: String,
    pub priority: String,
    pub author_id: String,
    pub author_name: String,
    pub server: String,
    pub guild_id: Option<String>,
    pub category: String,
    pub ticket_type: String,
    pub channel_id: Option<String>,
}

// ── Client ──

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    /// Helper : construit un `ApiClient` depuis le TypeMap Serenity.
    pub fn from_data(data: &serenity::prelude::TypeMap) -> Option<Self> {
        let grpc = data
            .get::<crate::shared::grpc_client::GrpcClientKey>()?
            .clone();
        Some(Self::new(grpc))
    }

    pub async fn list_tickets(&self) -> Result<Vec<Ticket>, String> {
        const PAGE_SIZE: i64 = 500;
        const MAX_PAGES: usize = 40;
        let mut out: Vec<Ticket> = Vec::new();
        let mut offset: i64 = 0;
        for _ in 0..MAX_PAGES {
            let req = proto::ListTicketsRequest {
                status: None,
                priority: None,
                search: None,
                author_id: None,
                limit: PAGE_SIZE,
                offset,
            };
            let list = crate::grpc_call!(self.grpc, tickets, list_tickets, req)?;
            let returned = list.tickets.len();
            out.extend(list.tickets.into_iter().map(proto_ticket_to_dto));
            if returned < PAGE_SIZE as usize {
                break;
            }
            offset += PAGE_SIZE;
        }
        Ok(out)
    }

    pub async fn create_ticket(&self, request: &CreateTicketRequest) -> Result<Ticket, String> {
        let req = proto::CreateTicketRequest {
            title: request.title.clone(),
            priority: request.priority.clone(),
            author_id: request.author_id.clone(),
            author_name: request.author_name.clone(),
            server: request.server.clone(),
            guild_id: request.guild_id.clone(),
            category: request.category.clone(),
            ticket_type: request.ticket_type.clone(),
            channel_id: request.channel_id.clone(),
        };
        let t = crate::grpc_call!(self.grpc, tickets, create_ticket, req)?;
        Ok(proto_ticket_to_dto(t))
    }

    pub async fn get_ticket(&self, id: &str) -> Result<TicketDetail, String> {
        let req = proto::GetTicketDetailRequest { id: id.to_string() };
        let detail = crate::grpc_call!(self.grpc, tickets, get_ticket_detail, req)?;
        Ok(proto_ticket_detail_to_dto(detail))
    }

    pub async fn reply_ticket(
        &self,
        ticket_id: &str,
        content: &str,
        author_name: &str,
        author_role: &str,
    ) -> Result<(), String> {
        let req = proto::ReplyTicketRequest {
            ticket_id: ticket_id.to_string(),
            content: content.to_string(),
            author_name: author_name.to_string(),
            author_role: author_role.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, tickets, reply_ticket, req)
    }

    pub async fn close_ticket(&self, id: &str) -> Result<(), String> {
        let req = proto::CloseTicketRequest { id: id.to_string() };
        crate::grpc_call!(@unit self.grpc, tickets, close_ticket, req)
    }

    pub async fn update_ticket_channel(
        &self,
        id: &str,
        voice_channel_id: Option<String>,
        invited_user_id: Option<String>,
    ) -> Result<(), String> {
        let req = proto::UpdateTicketChannelRequest {
            ticket_id: id.to_string(),
            voice_channel_id,
            invited_user_id,
        };
        crate::grpc_call!(@unit self.grpc, tickets, update_ticket_channel, req)
    }

    pub async fn update_ticket_sla(
        &self,
        id: &str,
        first_response_at: Option<&str>,
        resolved_at: Option<&str>,
        satisfaction_rating: Option<u8>,
    ) {
        let req = proto::UpdateSlaRequest {
            id: id.to_string(),
            first_response_at: first_response_at.map(|s| s.to_string()),
            resolved_at: resolved_at.map(|s| s.to_string()),
            satisfaction_rating: satisfaction_rating.map(|r| r as i32),
        };
        let id_owned = id.to_string();
        if let Err(e) = crate::grpc_call!(@raw_unit self.grpc, tickets, update_sla, req) {
            tracing::warn!(ticket_id = %id_owned, error = %grpc_err_to_string(e), "UpdateSla a echoue");
        }
    }
}

// ── Helpers proto -> DTO ──

fn proto_ticket_to_dto(t: proto::Ticket) -> Ticket {
    Ticket {
        id: t.id,
        title: t.title,
        status: t.status,
        priority: t.priority,
        author_id: t.author_id,
        author_name: t.author_name,
        assigned_to: t.assigned_to,
        server: t.server,
        category: t.category,
        ticket_type: Some(t.ticket_type),
        channel_id: t.channel_id,
        voice_channel_id: t.voice_channel_id,
        invited_user_id: t.invited_user_id,
        created_at: t.created_at,
        updated_at: t.updated_at,
        messages_count: t.messages_count,
        first_response_at: None,
        resolved_at: None,
        satisfaction_rating: None,
    }
}

fn proto_ticket_message_to_dto(m: proto::TicketMessage) -> TicketMessage {
    TicketMessage {
        author_name: m.author_name,
        author_role: m.author_role,
        content: m.content,
    }
}

fn proto_ticket_detail_to_dto(d: proto::TicketDetail) -> TicketDetail {
    TicketDetail {
        ticket: d
            .ticket
            .map(proto_ticket_to_dto)
            .unwrap_or_else(empty_ticket),
        messages: d
            .messages
            .into_iter()
            .map(proto_ticket_message_to_dto)
            .collect(),
    }
}

fn empty_ticket() -> Ticket {
    Ticket {
        id: String::new(),
        title: String::new(),
        status: String::new(),
        priority: String::new(),
        author_id: String::new(),
        author_name: String::new(),
        assigned_to: None,
        server: String::new(),
        category: String::new(),
        ticket_type: None,
        channel_id: None,
        voice_channel_id: None,
        invited_user_id: None,
        created_at: String::new(),
        updated_at: String::new(),
        messages_count: 0,
        first_response_at: None,
        resolved_at: None,
        satisfaction_rating: None,
    }
}

use crate::shared::grpc_client::grpc_err_to_string;
