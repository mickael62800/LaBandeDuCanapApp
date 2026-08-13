//! Implementation gRPC du `SecurityService` (Phase 7A).
//! Wrappe `ManageSecurityUseCase`.

use std::sync::Arc;

use platform_proto::sentinel::security::v1 as proto;
use platform_proto::sentinel::security::v1::security_service_server::SecurityService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::audit::security_event::SecurityEvent;
use platform_core::sentinel::domain::services::audit::security_analyzer::JoinInfo;
use platform_core::sentinel::ports::inbound::audit::manage_security::AnalyzeNewMemberCommand;
use platform_core::sentinel::ports::inbound::audit::manage_security::ManageSecurityUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_security::ReportSecurityEventCommand;
pub struct SecurityGrpc {
    pub uc: Arc<dyn ManageSecurityUseCase>,
}

#[tonic::async_trait]
impl SecurityService for SecurityGrpc {
    async fn report_event(
        &self,
        request: Request<proto::ReportEventRequest>,
    ) -> Result<Response<proto::SecurityEvent>, Status> {
        let req = request.into_inner();
        let event = self
            .uc
            .report_event(ReportSecurityEventCommand {
                guild_id: req.guild_id.into(),
                event_type: req.event_type,
                severity: req.severity,
                description: req.description,
                user_ids: req.user_ids,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(security_event_to_proto(event)))
    }

    async fn list_events(
        &self,
        request: Request<proto::ListEventsRequest>,
    ) -> Result<Response<proto::SecurityEventList>, Status> {
        let req = request.into_inner();
        let events = self
            .uc
            .list_events(req.guild_id.as_deref())
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityEventList {
            events: events.into_iter().map(security_event_to_proto).collect(),
        }))
    }

    async fn analyze_new_member(
        &self,
        request: Request<proto::AnalyzeNewMemberRequest>,
    ) -> Result<Response<proto::SecurityDecision>, Status> {
        let req = request.into_inner();
        let recent_joins = req
            .recent_joins
            .into_iter()
            .map(|j| JoinInfo {
                username: j.username,
                has_avatar: j.has_avatar,
                account_created_timestamp: j.account_created_timestamp,
            })
            .collect();
        let decision = self
            .uc
            .analyze_new_member(AnalyzeNewMemberCommand {
                guild_id: req.guild_id.into(),
                user_id: req.user_id.into(),
                username: req.username,
                has_avatar: req.has_avatar,
                account_created_timestamp: req.account_created_timestamp,
                is_bot: req.is_bot,
                recent_joins,
                is_velocity_raid: req.is_velocity_raid,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::SecurityDecision {
            is_raid: decision.is_raid,
            raid_score: decision.raid_score,
            is_suspicious_account: decision.is_suspicious_account,
            is_alt_account: decision.is_alt_account,
            alt_similar_to: decision.alt_similar_to,
            quarantine: decision.quarantine,
            send_captcha: decision.send_captcha,
            activate_lockdown: decision.activate_lockdown,
            slowmode_secs: decision.slowmode_secs,
            event_type: decision.event_type,
            event_description: decision.event_description,
            suggest_only: decision.suggest_only,
        }))
    }
}

fn security_event_to_proto(e: SecurityEvent) -> proto::SecurityEvent {
    proto::SecurityEvent {
        id: e.id.to_string(),
        guild_id: e.guild_id.into(),
        event_type: e.event_type,
        severity: e.severity,
        description: e.description,
        user_ids: e.user_ids,
        created_at: e.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
#[path = "tests/security.rs"]
mod tests;
