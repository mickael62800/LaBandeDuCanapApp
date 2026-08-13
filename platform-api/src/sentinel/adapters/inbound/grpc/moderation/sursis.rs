//! Implementation gRPC du `SursisService` (« ban en sursis »).
//!
//! Wrappe `ManageSursisUseCase`. Remplace les endpoints HTTP
//! `POST/GET /api/moderation/.../sursis...` appeles par moderation-bot.
//! Le delai d'appel est lu cote serveur dans la config guild (le bot ne le
//! fournit pas), comme le handler HTTP.

use std::sync::Arc;

use platform_proto::sentinel::sursis::v1 as proto;
use platform_proto::sentinel::sursis::v1::sursis_service_server::SursisService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::moderation::sursis::Sursis;
use platform_core::sentinel::domain::entities::moderation::sursis::SursisStatus;
use platform_core::sentinel::ports::inbound::moderation::manage_sursis::CreateSursisCommand;
use platform_core::sentinel::ports::inbound::moderation::manage_sursis::ManageSursisUseCase;
use platform_core::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

pub struct SursisGrpc {
    pub sursis_uc: Arc<dyn ManageSursisUseCase>,
    /// Lecture du delai d'appel (`sursis_appeal_days`) dans la config guild.
    pub bot_config_repo: Arc<dyn BotConfigRepository>,
}

#[tonic::async_trait]
impl SursisService for SursisGrpc {
    async fn create_sursis(
        &self,
        request: Request<proto::CreateSursisRequest>,
    ) -> Result<Response<proto::Sursis>, Status> {
        let req = request.into_inner();
        // Delai depuis la config (parametrable), defaut 7 jours — identique au HTTP.
        let days = platform_core::sentinel::domain::entities::system::bot_config::cfg_i64(
            &self
                .bot_config_repo
                .get_config(
                    &req.guild_id,
                    platform_core::sentinel::domain::entities::system::bot_names::MODERATION_BOT,
                )
                .await
                .unwrap_or_default(),
            "sursis_appeal_days",
            7,
        );

        let sursis = self
            .sursis_uc
            .create(CreateSursisCommand {
                guild_id: req.guild_id,
                user_id: req.user_id,
                username: req.username,
                moderator_id: req.moderator_id,
                moderator_name: req.moderator_name,
                reason: req.reason,
                saved_roles: req.saved_roles,
                channel_id: req.channel_id,
                days,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(sursis_to_proto(sursis)))
    }

    async fn get_sursis(
        &self,
        request: Request<proto::GetSursisRequest>,
    ) -> Result<Response<proto::Sursis>, Status> {
        let id = parse_uuid(&request.into_inner().id)?;
        let sursis = self
            .sursis_uc
            .get(id)
            .await
            .map_err(domain_to_status)?
            .ok_or_else(|| Status::not_found("Sursis introuvable"))?;
        Ok(Response::new(sursis_to_proto(sursis)))
    }

    async fn resolve_sursis(
        &self,
        request: Request<proto::ResolveSursisRequest>,
    ) -> Result<Response<proto::ResolveSursisResponse>, Status> {
        let req = request.into_inner();
        let id = parse_uuid(&req.id)?;
        let status = SursisStatus::from_str_lossy(&req.status)
            .ok_or_else(|| Status::invalid_argument(format!("statut invalide : {}", req.status)))?;
        // Gate sur l'existence de la ressource (comme le HTTP) avant de resoudre.
        self.sursis_uc
            .get(id)
            .await
            .map_err(domain_to_status)?
            .ok_or_else(|| Status::not_found("Sursis introuvable"))?;
        let claimed = self
            .sursis_uc
            .resolve(id, status)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ResolveSursisResponse { claimed }))
    }
}

fn parse_uuid(raw: &str) -> Result<uuid::Uuid, Status> {
    uuid::Uuid::parse_str(raw).map_err(|_| Status::invalid_argument("id invalide (UUID attendu)"))
}

fn sursis_to_proto(s: Sursis) -> proto::Sursis {
    proto::Sursis {
        id: s.id.to_string(),
        user_id: s.user_id,
        username: s.username,
        reason: s.reason,
        saved_roles: s.saved_roles,
        channel_id: s.channel_id,
        status: s.status.as_str().to_string(),
        expires_at: s.expires_at.to_rfc3339(),
    }
}

#[cfg(test)]
#[path = "tests/sursis.rs"]
mod tests;
