//! Implementation gRPC du `AnnouncementsService`.
//!
//! Wrappe la partie « report » de `ManageAnnouncementsUseCase`. Remplace les
//! endpoints HTTP `/api/announcements/internal/{runs/.../result,button-click}`
//! appeles par le bot apres publication.

use std::sync::Arc;

use platform_proto::sentinel::announcements::v1 as proto;
use platform_proto::sentinel::announcements::v1::announcements_service_server::AnnouncementsService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::community::announcement::ChannelPostResult;
use platform_core::sentinel::ports::inbound::community::manage_announcements::ManageAnnouncementsUseCase;

pub struct AnnouncementsGrpc {
    pub uc: Arc<dyn ManageAnnouncementsUseCase>,
}

#[tonic::async_trait]
impl AnnouncementsService for AnnouncementsGrpc {
    async fn record_run_result(
        &self,
        request: Request<proto::RecordRunResultRequest>,
    ) -> Result<Response<proto::AnnouncementsAck>, Status> {
        let req = request.into_inner();
        let run_id = parse_uuid("run_id", &req.run_id)?;
        let channels_posted = req
            .channels_posted
            .into_iter()
            .map(|c| ChannelPostResult {
                channel_id: c.channel_id,
                message_id: c.message_id,
                success: c.success,
                error: c.error,
            })
            .collect();
        self.uc
            .record_run_result(run_id, channels_posted)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AnnouncementsAck {}))
    }

    async fn record_button_click(
        &self,
        request: Request<proto::RecordButtonClickRequest>,
    ) -> Result<Response<proto::AnnouncementsAck>, Status> {
        let req = request.into_inner();
        let announcement_id = parse_uuid("announcement_id", &req.announcement_id)?;
        let run_id = req
            .run_id
            .as_deref()
            .map(|s| parse_uuid("run_id", s))
            .transpose()?;
        self.uc
            .record_button_interaction(
                announcement_id,
                run_id,
                req.user_id,
                req.user_name,
                req.button_custom_id,
                req.button_label,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AnnouncementsAck {}))
    }
}

fn parse_uuid(field: &str, raw: &str) -> Result<uuid::Uuid, Status> {
    uuid::Uuid::parse_str(raw)
        .map_err(|_| Status::invalid_argument(format!("{field} invalide (UUID attendu)")))
}
