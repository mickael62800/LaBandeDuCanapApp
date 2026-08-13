//! Implementation gRPC du `ConfessionsService`.
//!
//! Wrappe `ManageConfessionsUseCase`. Remplace les endpoints HTTP
//! `/api/confessions/...` appeles par confessions-bot (creation, reponses,
//! signalements, refs Discord, config). Les operations purement web
//! (edit/list-replies/reports/resolve) restent en HTTP.

use std::sync::Arc;

use platform_proto::sentinel::confessions::v1 as proto;
use platform_proto::sentinel::confessions::v1::confessions_service_server::ConfessionsService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::community::confession::Confession;
use platform_core::sentinel::domain::entities::community::confession::ConfessionConfig;
use platform_core::sentinel::domain::entities::community::confession::ConfessionReply;
use platform_core::sentinel::ports::inbound::community::manage_confessions::CreateConfessionCommand;
use platform_core::sentinel::ports::inbound::community::manage_confessions::CreateReplyCommand;
use platform_core::sentinel::ports::inbound::community::manage_confessions::CreateReportCommand;
use platform_core::sentinel::ports::inbound::community::manage_confessions::ManageConfessionsUseCase;

pub struct ConfessionsGrpc {
    pub uc: Arc<dyn ManageConfessionsUseCase>,
}

#[tonic::async_trait]
impl ConfessionsService for ConfessionsGrpc {
    async fn create_confession(
        &self,
        request: Request<proto::CreateConfessionRequest>,
    ) -> Result<Response<proto::Confession>, Status> {
        let req = request.into_inner();
        let c = self
            .uc
            .create(CreateConfessionCommand {
                guild_id: req.guild_id,
                author_user_id: req.author_user_id,
                content: req.content,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(confession_to_proto(c)))
    }

    async fn get_confession(
        &self,
        request: Request<proto::GetConfessionRequest>,
    ) -> Result<Response<proto::Confession>, Status> {
        let id = parse_uuid(&request.into_inner().id)?;
        let c = self.uc.get(id).await.map_err(domain_to_status)?;
        Ok(Response::new(confession_to_proto(c)))
    }

    async fn list_confessions(
        &self,
        request: Request<proto::ListConfessionsRequest>,
    ) -> Result<Response<proto::ConfessionList>, Status> {
        let req = request.into_inner();
        let list = self
            .uc
            .list(&req.guild_id, req.limit, req.include_deleted)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ConfessionList {
            confessions: list.into_iter().map(confession_to_proto).collect(),
        }))
    }

    async fn delete_confession(
        &self,
        request: Request<proto::DeleteConfessionRequest>,
    ) -> Result<Response<proto::Confession>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;
        let c = self
            .uc
            .delete(id, req.deleted_by, req.reason)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(confession_to_proto(c)))
    }

    async fn update_message_refs(
        &self,
        request: Request<proto::UpdateMessageRefsRequest>,
    ) -> Result<Response<proto::ConfessionsAck>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;
        self.uc
            .update_message_refs(id, req.message_id, req.channel_id, req.thread_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ConfessionsAck {}))
    }

    async fn create_reply(
        &self,
        request: Request<proto::CreateReplyRequest>,
    ) -> Result<Response<proto::ConfessionReply>, Status> {
        let req = request.into_inner();
        let confession_id = parse_uuid(&req.confession_id)?;
        let r = self
            .uc
            .create_reply(CreateReplyCommand {
                confession_id,
                author_user_id: req.author_user_id,
                content: req.content,
                is_anonymous: req.is_anonymous,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(reply_to_proto(r)))
    }

    async fn update_reply_message_id(
        &self,
        request: Request<proto::UpdateReplyMessageIdRequest>,
    ) -> Result<Response<proto::ConfessionsAck>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.reply_id)?;
        self.uc
            .update_reply_message_id(id, req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ConfessionsAck {}))
    }

    async fn create_report(
        &self,
        request: Request<proto::CreateReportRequest>,
    ) -> Result<Response<proto::ConfessionsAck>, Status> {
        let req = request.into_inner();
        let confession_id = req.confession_id.as_deref().map(parse_uuid).transpose()?;
        let reply_id = req.reply_id.as_deref().map(parse_uuid).transpose()?;
        self.uc
            .create_report(CreateReportCommand {
                guild_id: req.guild_id,
                confession_id,
                reply_id,
                reporter_user_id: req.reporter_user_id,
                reason: req.reason,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ConfessionsAck {}))
    }

    async fn get_config(
        &self,
        request: Request<proto::GetConfigRequest>,
    ) -> Result<Response<proto::ConfessionConfig>, Status> {
        let cfg = self
            .uc
            .get_config(&request.into_inner().guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(config_to_proto(cfg)))
    }
}

fn parse_uuid(raw: &str) -> Result<uuid::Uuid, Status> {
    uuid::Uuid::parse_str(raw).map_err(|_| Status::invalid_argument("id invalide (UUID attendu)"))
}

fn confession_to_proto(c: Confession) -> proto::Confession {
    proto::Confession {
        id: c.id.to_string(),
        guild_id: c.guild_id,
        public_number: c.public_number,
        author_user_id: c.author_user_id,
        content: c.content,
        message_id: c.message_id,
        channel_id: c.channel_id,
        thread_id: c.thread_id,
        created_at: c.created_at.to_rfc3339(),
    }
}

fn reply_to_proto(r: ConfessionReply) -> proto::ConfessionReply {
    proto::ConfessionReply {
        id: r.id.to_string(),
        confession_id: r.confession_id.to_string(),
        public_number: r.public_number,
        author_user_id: r.author_user_id,
        content: r.content,
        is_anonymous: r.is_anonymous,
        message_id: r.message_id,
        created_at: r.created_at.to_rfc3339(),
    }
}

fn config_to_proto(c: ConfessionConfig) -> proto::ConfessionConfig {
    proto::ConfessionConfig {
        guild_id: c.guild_id,
        enabled: c.enabled,
        channel_id: c.channel_id,
        panel_message_id: c.panel_message_id,
        cooldown_secs: c.cooldown_secs,
        max_per_day: c.max_per_day,
        quota_window_hours: c.quota_window_hours,
        min_chars: c.min_chars,
        max_chars: c.max_chars,
        automod_enabled: c.automod_enabled,
        banned_user_ids: c.banned_user_ids,
        updated_at: c.updated_at.to_rfc3339(),
    }
}

#[cfg(test)]
#[path = "tests/confessions.rs"]
mod tests;
