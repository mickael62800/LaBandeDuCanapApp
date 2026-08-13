//! Implementation gRPC du `DiscordActionMessagesService`.
//!
//! Mapping `action_id <-> message(s) Discord` (table `discord_action_messages`).
//! Ressource de sync transverse, partagee par plusieurs modules bot
//! (automod, tickets, voice). Wrappe `ManageDiscordActionMessagesUseCase`.

use std::sync::Arc;

use platform_proto::sentinel::discord_messages::v1 as proto;
use platform_proto::sentinel::discord_messages::v1::discord_action_messages_service_server::DiscordActionMessagesService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::audit::discord_action_message::DiscordActionMessage;
use platform_core::sentinel::domain::entities::audit::discord_action_message::NewDiscordActionMessage;
use platform_core::sentinel::ports::inbound::audit::manage_discord_action_messages::ManageDiscordActionMessagesUseCase;

pub struct DiscordActionMessagesGrpc {
    pub uc: Arc<dyn ManageDiscordActionMessagesUseCase>,
}

#[tonic::async_trait]
impl DiscordActionMessagesService for DiscordActionMessagesGrpc {
    async fn register(
        &self,
        request: Request<proto::RegisterRequest>,
    ) -> Result<Response<proto::RegisterResponse>, Status> {
        let req = request.into_inner();
        let action_id = uuid::Uuid::parse_str(&req.action_id)
            .map_err(|_| Status::invalid_argument("action_id invalide (UUID attendu)"))?;
        self.uc
            .register(NewDiscordActionMessage {
                action_id,
                kind: req.kind,
                guild_id: req.guild_id.into(),
                channel_id: req.channel_id.into(),
                message_id: req.message_id.into(),
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::RegisterResponse {}))
    }

    async fn list_for_action(
        &self,
        request: Request<proto::ListForActionRequest>,
    ) -> Result<Response<proto::ActionMessageList>, Status> {
        let action_id = uuid::Uuid::parse_str(&request.into_inner().action_id)
            .map_err(|_| Status::invalid_argument("action_id invalide (UUID attendu)"))?;
        let list = self
            .uc
            .list_for_action(action_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ActionMessageList {
            messages: list.into_iter().map(action_message_to_proto).collect(),
        }))
    }
}

fn action_message_to_proto(m: DiscordActionMessage) -> proto::ActionMessage {
    proto::ActionMessage {
        action_id: m.action_id.to_string(),
        kind: m.kind,
        guild_id: m.guild_id.to_string(),
        channel_id: m.channel_id.to_string(),
        message_id: m.message_id.to_string(),
        posted_at: m.posted_at.to_rfc3339(),
        last_edited_at: m.last_edited_at.map(|d| d.to_rfc3339()),
    }
}

#[cfg(test)]
#[path = "tests/action_messages.rs"]
mod tests;
