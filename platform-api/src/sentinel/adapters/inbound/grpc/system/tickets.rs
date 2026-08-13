//! Implementation gRPC du `TicketsService`. Wrappe le use-case
//! `ManageTicketsUseCase` deja utilise par les handlers HTTP.

use std::sync::Arc;

use platform_proto::sentinel::tickets::v1 as proto;
use platform_proto::sentinel::tickets::v1::tickets_service_server::TicketsService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::system::ticket::Ticket;
use platform_core::sentinel::domain::entities::system::ticket::TicketDetail;
use platform_core::sentinel::domain::entities::system::ticket::TicketMessage;
use platform_core::sentinel::ports::inbound::system::manage_tickets::AssignTicketCommand;
use platform_core::sentinel::ports::inbound::system::manage_tickets::CreateTicketCommand;
use platform_core::sentinel::ports::inbound::system::manage_tickets::ManageTicketsUseCase;
use platform_core::sentinel::ports::inbound::system::manage_tickets::ReplyTicketCommand;
use platform_core::sentinel::ports::inbound::system::manage_tickets::UpdateTicketChannelCommand;
pub struct TicketsGrpc {
    pub tickets_uc: Arc<dyn ManageTicketsUseCase>,
}

#[tonic::async_trait]
impl TicketsService for TicketsGrpc {
    async fn list_tickets(
        &self,
        request: Request<proto::ListTicketsRequest>,
    ) -> Result<Response<proto::TicketList>, Status> {
        let req = request.into_inner();
        let limit = if req.limit <= 0 {
            50
        } else {
            req.limit.min(200)
        };
        let offset = req.offset.max(0);
        let tickets = self
            .tickets_uc
            .list_tickets(
                req.status,
                req.priority,
                req.search,
                req.author_id,
                limit,
                offset,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TicketList {
            tickets: tickets.into_iter().map(ticket_to_proto).collect(),
        }))
    }

    async fn get_ticket_detail(
        &self,
        request: Request<proto::GetTicketDetailRequest>,
    ) -> Result<Response<proto::TicketDetail>, Status> {
        let detail = self
            .tickets_uc
            .get_ticket_detail(&request.into_inner().id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(ticket_detail_to_proto(detail)))
    }

    async fn create_ticket(
        &self,
        request: Request<proto::CreateTicketRequest>,
    ) -> Result<Response<proto::Ticket>, Status> {
        let req = request.into_inner();
        let ticket = self
            .tickets_uc
            .create_ticket(CreateTicketCommand {
                title: req.title,
                priority: req.priority,
                author_id: req.author_id,
                author_name: req.author_name,
                server: req.server,
                guild_id: req.guild_id,
                category: req.category,
                ticket_type: req.ticket_type,
                channel_id: req.channel_id,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(ticket_to_proto(ticket)))
    }

    async fn reply_ticket(
        &self,
        request: Request<proto::ReplyTicketRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.tickets_uc
            .reply_ticket(ReplyTicketCommand {
                ticket_id: req.ticket_id,
                content: req.content,
                author_name: req.author_name,
                author_role: req.author_role,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn close_ticket(
        &self,
        request: Request<proto::CloseTicketRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        self.tickets_uc
            .close_ticket(&request.into_inner().id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_status(
        &self,
        request: Request<proto::UpdateStatusRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.tickets_uc
            .update_status(&req.id, &req.status)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn assign_ticket(
        &self,
        request: Request<proto::AssignTicketRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.tickets_uc
            .assign_ticket(AssignTicketCommand {
                ticket_id: req.ticket_id,
                assignee: req.assignee,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_ticket_channel(
        &self,
        request: Request<proto::UpdateTicketChannelRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.tickets_uc
            .update_ticket_channel(UpdateTicketChannelCommand {
                ticket_id: req.ticket_id,
                voice_channel_id: req.voice_channel_id,
                invited_user_id: req.invited_user_id,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_priority(
        &self,
        request: Request<proto::UpdatePriorityRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let id = crate::sentinel::adapters::inbound::grpc::parse_uuid(&req.id)?;
        self.tickets_uc
            .update_priority(id, &req.priority)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn update_sla(
        &self,
        request: Request<proto::UpdateSlaRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        let id = crate::sentinel::adapters::inbound::grpc::parse_uuid(&req.id)?;
        self.tickets_uc
            .update_sla(
                id,
                req.first_response_at.as_deref(),
                req.resolved_at.as_deref(),
                req.satisfaction_rating,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::Empty {}))
    }
}

fn ticket_to_proto(t: Ticket) -> proto::Ticket {
    proto::Ticket {
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

fn ticket_message_to_proto(m: TicketMessage) -> proto::TicketMessage {
    proto::TicketMessage {
        id: m.id.to_string(),
        ticket_id: m.ticket_id.to_string(),
        author_name: m.author_name,
        author_role: m.author_role,
        content: m.content,
        created_at: m.created_at.to_rfc3339(),
    }
}

fn ticket_detail_to_proto(d: TicketDetail) -> proto::TicketDetail {
    proto::TicketDetail {
        ticket: Some(ticket_to_proto(d.ticket)),
        messages: d
            .messages
            .into_iter()
            .map(ticket_message_to_proto)
            .collect(),
    }
}

#[cfg(test)]
#[path = "tests/tickets.rs"]
mod tests;
