//! Implementation gRPC du `SecurityStateService`.
//!
//! Miroir de persistance de l'etat de securite actif (quarantaines, slowmode,
//! lockdown). Wrappe les use cases du domaine SYSTEM. Remplace les endpoints
//! HTTP `/api/security/{quarantine,slowmode,lockdown}` appeles par le bot.

use std::sync::Arc;

use platform_proto::sentinel::security_state::v1 as proto;
use platform_proto::sentinel::security_state::v1::security_state_service_server::SecurityStateService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::ports::inbound::system::manage_lockdown::ManageLockdownUseCase;
use platform_core::sentinel::ports::inbound::system::manage_quarantine::ManageQuarantineUseCase;
use platform_core::sentinel::ports::inbound::system::manage_slowmode::ManageSlowmodeUseCase;

pub struct SecurityStateGrpc {
    pub quarantine_uc: Arc<dyn ManageQuarantineUseCase>,
    pub slowmode_uc: Arc<dyn ManageSlowmodeUseCase>,
    pub lockdown_uc: Arc<dyn ManageLockdownUseCase>,
}

#[tonic::async_trait]
impl SecurityStateService for SecurityStateGrpc {
    async fn mark_quarantine(
        &self,
        request: Request<proto::MarkQuarantineRequest>,
    ) -> Result<Response<proto::MarkQuarantineAck>, Status> {
        let req = request.into_inner();
        // Zero signifie « pas de valeur imposee » : le reglage de la guilde
        // fait foi. C'est le cas normal, le bot n'ayant pas a connaitre le
        // delai pour poser une quarantaine.
        let impose = Some(req.timeout_secs).filter(|v| *v > 0);
        let applique = self
            .quarantine_uc
            .quarantine_user(&req.guild_id, &req.user_id, impose)
            .await
            .map_err(domain_to_status)?;
        // Le reglage retenu repart vers le bot : c'est lui qui ecrit le message
        // prive, et il doit y annoncer le delai reel.
        Ok(Response::new(proto::MarkQuarantineAck {
            timeout_secs: applique.timeout_secs,
            reminder_secs: applique.reminder_secs,
            kick_enabled: applique.kick_enabled,
        }))
    }

    async fn get_quarantine_settings(
        &self,
        request: Request<proto::GetQuarantineSettingsRequest>,
    ) -> Result<Response<proto::MarkQuarantineAck>, Status> {
        let req = request.into_inner();
        let reglage = self
            .quarantine_uc
            .settings(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::MarkQuarantineAck {
            timeout_secs: reglage.timeout_secs,
            reminder_secs: reglage.reminder_secs,
            kick_enabled: reglage.kick_enabled,
        }))
    }

    async fn lift_quarantine(
        &self,
        request: Request<proto::LiftQuarantineRequest>,
    ) -> Result<Response<proto::SecurityStateAck>, Status> {
        let req = request.into_inner();
        self.quarantine_uc
            .lift(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityStateAck {}))
    }

    async fn list_active_quarantines(
        &self,
        _request: Request<proto::ListActiveQuarantinesRequest>,
    ) -> Result<Response<proto::ActiveQuarantineList>, Status> {
        let rows = self
            .quarantine_uc
            .list_active()
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ActiveQuarantineList {
            entries: rows
                .into_iter()
                .map(|q| proto::ActiveQuarantineEntry {
                    guild_id: q.guild_id,
                    user_id: q.user_id,
                })
                .collect(),
        }))
    }

    async fn mark_slowmode(
        &self,
        request: Request<proto::MarkSlowmodeRequest>,
    ) -> Result<Response<proto::SecurityStateAck>, Status> {
        let req = request.into_inner();
        // Le champ est un JSON opaque cote metier : on le parse en Value, comme
        // le faisait le handler HTTP (qui recevait deja un serde_json::Value).
        let previous_states = parse_json_field(&req.previous_states_json)?;
        self.slowmode_uc
            .activate(
                &req.guild_id,
                previous_states,
                req.duration_secs,
                req.imposed_rate,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityStateAck {}))
    }

    async fn mark_lockdown(
        &self,
        request: Request<proto::MarkLockdownRequest>,
    ) -> Result<Response<proto::SecurityStateAck>, Status> {
        let req = request.into_inner();
        let saved_states = parse_json_field(&req.saved_states_json)?;
        self.lockdown_uc
            .activate(&req.guild_id, saved_states, req.duration_secs)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityStateAck {}))
    }
}

/// Parse un champ JSON transporte en string. Vide -> `null` (accepte par les
/// use cases). Chaine invalide -> `InvalidArgument` explicite.
fn parse_json_field(raw: &str) -> Result<serde_json::Value, Status> {
    if raw.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(raw).map_err(|e| Status::invalid_argument(format!("JSON invalide : {e}")))
}

#[cfg(test)]
#[path = "tests/security_state.rs"]
mod tests;
