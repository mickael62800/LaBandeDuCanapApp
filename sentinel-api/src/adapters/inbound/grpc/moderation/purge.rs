//! gRPC handler des purges administratives (`/cleanup` cote bot).
//!
//! Miroir exact des trois handlers HTTP `DELETE /api/purge/*` : memes use
//! cases, memes validations, meme broadcast. Le HTTP reste en place pour le
//! dashboard web ; c'est le bot qui bascule ici.

use std::sync::Arc;

use sentinel_proto::purge::v1 as proto;
use sentinel_proto::purge::v1::purge_service_server::PurgeService;
use tonic::Request;
use tonic::Response;
use tonic::Status;
use tracing::info;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use crate::adapters::outbound::ws::broadcaster::EventBroadcaster;
use sentinel_core::domain::entities::moderation::purge::{
    validate_purge_days_allow_zero, validate_purge_days_strictly_positive,
};
use sentinel_core::ports::inbound::audit::manage_audit_logs::ManageAuditLogsUseCase;
use sentinel_core::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_core::ports::outbound::ops::log_repository::LogRepository;

pub struct PurgeGrpc {
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub log_repo: Arc<dyn LogRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
}

impl PurgeGrpc {
    /// Diffuse le meme event que le chemin HTTP : le dashboard web ecoute
    /// `purge_completed` et doit se rafraichir quel que soit le transport
    /// utilise par l'appelant.
    fn broadcast(&self, kind: &str, guild_id: Option<&str>, days: i32, deleted: u64) {
        self.broadcaster.broadcast(
            "purge_completed",
            serde_json::json!({
                "type": kind,
                "guild_id": guild_id,
                "days": days,
                "deleted": deleted,
            }),
        );
    }
}

/// Refuse un identifiant de guild vide ou non-snowflake.
fn require_guild(guild_id: &str) -> Result<(), Status> {
    let ok = (17..=20).contains(&guild_id.len()) && guild_id.chars().all(|c| c.is_ascii_digit());
    if ok {
        Ok(())
    } else {
        Err(Status::invalid_argument("guild_id invalide"))
    }
}

#[tonic::async_trait]
impl PurgeService for PurgeGrpc {
    async fn purge_infractions(
        &self,
        request: Request<proto::PurgeByDaysRequest>,
    ) -> Result<Response<proto::PurgeResponse>, Status> {
        let req = request.into_inner();
        require_guild(&req.guild_id)?;
        // `days = 0` est ACCEPTE ici, et uniquement ici : purger toutes les
        // infractions d'un serveur est une operation legitime de remise a zero.
        validate_purge_days_allow_zero(req.days).map_err(Status::invalid_argument)?;

        let deleted = self
            .infractions_uc
            .delete_older_than_days(&req.guild_id, req.days)
            .await
            .map_err(domain_to_status)?;
        info!(guild_id = %req.guild_id, days = req.days, deleted, "Purge infractions (gRPC)");
        self.broadcast("infractions", Some(&req.guild_id), req.days, deleted);
        Ok(Response::new(proto::PurgeResponse { deleted }))
    }

    async fn purge_audit_logs(
        &self,
        request: Request<proto::PurgeByDaysRequest>,
    ) -> Result<Response<proto::PurgeResponse>, Status> {
        let req = request.into_inner();
        require_guild(&req.guild_id)?;
        validate_purge_days_strictly_positive(req.days).map_err(Status::invalid_argument)?;

        let deleted = self
            .audit_logs_uc
            .delete_older_than_days(&req.guild_id, req.days)
            .await
            .map_err(domain_to_status)?;
        info!(guild_id = %req.guild_id, days = req.days, deleted, "Purge audit logs (gRPC)");
        self.broadcast("audit_logs", Some(&req.guild_id), req.days, deleted);
        Ok(Response::new(proto::PurgeResponse { deleted }))
    }

    async fn purge_logs(
        &self,
        request: Request<proto::PurgeLogsRequest>,
    ) -> Result<Response<proto::PurgeResponse>, Status> {
        let req = request.into_inner();
        // Portee GLOBALE (toutes guilds) : pas de guild_id a valider, mais
        // `days = 0` reste interdit — effacer tous les logs applicatifs par
        // une valeur par defaut serait trop facile.
        validate_purge_days_strictly_positive(req.days).map_err(Status::invalid_argument)?;

        let deleted = self
            .log_repo
            .delete_older_than_days(req.days)
            .await
            .map_err(domain_to_status)?;
        info!(days = req.days, deleted, "Purge logs systeme (gRPC)");
        self.broadcast("logs", None, req.days, deleted);
        Ok(Response::new(proto::PurgeResponse { deleted }))
    }
}
