//! Implementation gRPC du `AgeGateService`.
//!
//! Wrappe `EvaluateAgeDeclarationUseCase` (decision) + `AgeBanRepository`
//! (enregistrement). Remplace les endpoints HTTP `POST /api/welcome/{g}/age-check`
//! et `POST /api/age-bans` appeles par welcome-bot.

use std::sync::Arc;

use platform_proto::sentinel::age_gate::v1 as proto;
use platform_proto::sentinel::age_gate::v1::age_gate_service_server::AgeGateService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::community::age_ban::AgeBan;
use platform_core::sentinel::domain::entities::community::age_ban::AgeBanStatus;
use platform_core::sentinel::domain::entities::community::age_check::AgeCheckDecision;
use platform_core::sentinel::ports::inbound::community::evaluate_age_declaration::EvaluateAgeDeclarationUseCase;
use platform_core::sentinel::ports::outbound::community::age_ban_repository::AgeBanRepository;

pub struct AgeGateGrpc {
    pub age_check_uc: Arc<dyn EvaluateAgeDeclarationUseCase>,
    pub age_ban_repo: Arc<dyn AgeBanRepository>,
}

#[tonic::async_trait]
impl AgeGateService for AgeGateGrpc {
    async fn check_age(
        &self,
        request: Request<proto::CheckAgeRequest>,
    ) -> Result<Response<proto::AgeCheckDecision>, Status> {
        let req = request.into_inner();
        let decision = self
            .age_check_uc
            .evaluate(&req.guild_id, &req.user_id, req.declared_age)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(match decision {
            AgeCheckDecision::Grant => proto::AgeCheckDecision {
                grant: true,
                years: 0,
                unban_at: String::new(),
                reason: String::new(),
            },
            AgeCheckDecision::Ban {
                years,
                unban_at,
                reason,
            } => proto::AgeCheckDecision {
                grant: false,
                years,
                unban_at: unban_at.to_rfc3339(),
                reason,
            },
        }))
    }

    async fn record_age_ban(
        &self,
        request: Request<proto::RecordAgeBanRequest>,
    ) -> Result<Response<proto::AgeGateAck>, Status> {
        let req = request.into_inner();
        let unban_at = chrono::DateTime::parse_from_rfc3339(&req.unban_at)
            .map_err(|_| Status::invalid_argument("unban_at invalide (RFC3339 attendu)"))?
            .with_timezone(&chrono::Utc);
        let ban = AgeBan {
            id: uuid::Uuid::new_v4(),
            guild_id: req.guild_id,
            user_id: req.user_id,
            declared_age: req.declared_age,
            banned_at: chrono::Utc::now(),
            unban_at,
            status: AgeBanStatus::Pending,
            lifted_at: None,
        };
        self.age_ban_repo
            .create(&ban)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::AgeGateAck {}))
    }
}
