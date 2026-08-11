//! Implementation gRPC du `ModerationService` (v1).
//!
//! Scope reduit volontairement (cf. moderation.proto) :
//! - `LogAction` : hot path appele a chaque sanction.
//! - `GetHistory` : consultation frequente.
//!
//! Les autres methodes du moderation-bot (evidence/review/modstats/pending)
//! continueront a passer par HTTP tant que le `ManageModerationUseCase`
//! n'expose pas ces operations.

use std::sync::Arc;

use sentinel_proto::moderation::v1 as proto;
use sentinel_proto::moderation::v1::moderation_service_server::ModerationService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::adapters::inbound::grpc::errors::domain_to_status;
use sentinel_core::domain::entities::moderation::action::applied::ModerationAction;
use sentinel_core::domain::entities::moderation::action::applied::UserModerationHistory;
use sentinel_core::ports::inbound::moderation::assess_target_risk::{
    AssessTargetRiskCommand, AssessTargetRiskUseCase,
};
use sentinel_core::ports::inbound::moderation::cancel_action::{
    CancelModerationActionUseCase, CancelOutcome,
};
use sentinel_core::ports::inbound::moderation::manage_infractions::ManageInfractionsUseCase;
use sentinel_core::ports::inbound::moderation::manage_moderation::LogModerationCommand;
use sentinel_core::ports::inbound::moderation::manage_moderation::ManageModerationUseCase;
use sentinel_core::ports::inbound::moderation::read_modstats::ReadModstatsUseCase;
use sentinel_core::ports::outbound::moderation::evidence_repository::EvidenceRepository;
use sentinel_core::ports::outbound::moderation::pending_action_repository::PendingActionRepository;
use sentinel_core::ports::outbound::moderation::review_repository::ReviewRepository;
pub struct ModerationGrpc {
    pub moderation_uc: Arc<dyn ManageModerationUseCase>,
    /// Annulation d'une action (/unwarn). Meme use case que le HTTP.
    pub cancel_action_uc: Arc<dyn CancelModerationActionUseCase>,
    // ── Ports du dossier de moderation (ex-HTTP) ──
    pub assess_target_risk_uc: Arc<dyn AssessTargetRiskUseCase>,
    pub modstats_uc: Arc<dyn ReadModstatsUseCase>,
    pub evidence_repo: Arc<dyn EvidenceRepository>,
    pub review_repo: Arc<dyn ReviewRepository>,
    pub pending_action_repo: Arc<dyn PendingActionRepository>,
    pub infractions_uc: Arc<dyn ManageInfractionsUseCase>,
    pub manage_reminders_uc: Arc<
        dyn sentinel_core::ports::inbound::moderation::manage_reminders::ManageRemindersUseCase,
    >,
}

#[tonic::async_trait]
impl ModerationService for ModerationGrpc {
    async fn log_action(
        &self,
        request: Request<proto::LogActionRequest>,
    ) -> Result<Response<proto::ModerationAction>, Status> {
        let req = request.into_inner();
        let skip_strike = req.skip_strike;
        let cmd = LogModerationCommand {
            guild_id: req.guild_id.clone().into(),
            channel_id: req.channel_id.clone().into(),
            moderator_id: req.moderator_id.clone(),
            moderator_name: req.moderator_name.clone(),
            target_id: req.target_id.clone(),
            target_name: req.target_name.clone(),
            action_type: req.action_type.clone(),
            reason: req.reason.clone(),
            gravity: req.gravity.clone(),
            duration: req.duration,
        };

        // skip_strike : sanction d'escalade auto deja adossee a un strike compte.
        // On journalise l'action SANS rejouer le strike (anti double-strike) en
        // passant par `log_action` plutot que `log_action_with_strike`.
        let (proto_action, action_id) = if skip_strike {
            let action = self
                .moderation_uc
                .log_action(cmd)
                .await
                .map_err(domain_to_status)?;
            let action_id = action.id;
            (moderation_action_to_proto(action), action_id)
        } else {
            // Phase 7B : orchestration atomique action+strike via le service.
            let logged = self
                .moderation_uc
                .log_action_with_strike(cmd)
                .await
                .map_err(domain_to_status)?;
            let action_id = logged.action.id;
            let mut pa = moderation_action_to_proto(logged.action);
            if let Some(sr) = logged.strike {
                pa.strikes_count = Some(sr.active_count);
                pa.escalation_action = sr.escalation_action;
                pa.escalation_duration = sr.escalation_duration;
            }
            (pa, action_id)
        };

        if let Some(duration_secs) = req.duration {
            let _ = self
                .manage_reminders_uc
                .create_reminder(
                    sentinel_core::ports::inbound::moderation::manage_reminders::CreateReminderCommand {
                        guild_id: req.guild_id.into(),
                        moderator_id: req.moderator_id,
                        moderator_name: req.moderator_name,
                        target_id: req.target_id,
                        target_name: req.target_name,
                        action_type: req.action_type,
                        reason: req.reason,
                        action_id,
                        duration_secs,
                        remind_before_secs: 0,
                    },
                )
                .await
                .map_err(domain_to_status)?;
        }

        Ok(Response::new(proto_action))
    }

    async fn get_history(
        &self,
        request: Request<proto::GetHistoryRequest>,
    ) -> Result<Response<proto::UserHistory>, Status> {
        let req = request.into_inner();
        let history = self
            .moderation_uc
            .get_history(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(user_history_to_proto(history)))
    }

    async fn cancel_action(
        &self,
        request: Request<proto::CancelActionRequest>,
    ) -> Result<Response<proto::CancelActionResponse>, Status> {
        let req = request.into_inner();
        let action_id = req
            .action_id
            .parse()
            .map_err(|_| Status::invalid_argument("action_id doit etre un UUID"))?;
        // Meme use case que le handler HTTP : l'effet Discord inverse et
        // l'annulation du rappel d'auto-unban ne peuvent pas diverger selon
        // le transport.
        let outcome = self
            .cancel_action_uc
            .cancel(action_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CancelActionResponse {
            cancelled: outcome == CancelOutcome::Cancelled,
        }))
    }

    // ── Garde-fous UX ──

    async fn assess_target_risk(
        &self,
        request: Request<proto::AssessTargetRiskRequest>,
    ) -> Result<Response<proto::TargetRiskDecision>, Status> {
        let req = request.into_inner();
        let d = self
            .assess_target_risk_uc
            .assess(AssessTargetRiskCommand {
                guild_id: req.guild_id,
                account_age_days: req.account_age_days,
                is_bot: req.is_bot,
                has_mod_perms: req.has_mod_perms,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::TargetRiskDecision {
            risky: d.risky,
            reason: d.reason,
        }))
    }

    async fn count_moderator_actions(
        &self,
        request: Request<proto::CountModeratorActionsRequest>,
    ) -> Result<Response<proto::CountModeratorActionsResponse>, Status> {
        let req = request.into_inner();
        let count = self
            .moderation_uc
            .count_recent_mod_actions(&req.guild_id, &req.moderator_id, req.window_secs as i64)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CountModeratorActionsResponse {
            count: count.max(0) as u32,
        }))
    }

    // ── Preuves ──

    async fn add_evidence(
        &self,
        request: Request<proto::AddEvidenceRequest>,
    ) -> Result<Response<proto::EvidenceEntry>, Status> {
        let req = request.into_inner();
        let action_id = parse_uuid_arg(&req.action_id, "action_id")?;
        let e = self
            .evidence_repo
            .add(
                action_id,
                &req.url,
                req.description.as_deref(),
                &req.uploaded_by,
                &req.uploaded_by_name,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(evidence_to_proto(e)))
    }

    async fn list_evidence(
        &self,
        request: Request<proto::ListEvidenceRequest>,
    ) -> Result<Response<proto::ListEvidenceResponse>, Status> {
        let req = request.into_inner();
        let action_id = parse_uuid_arg(&req.action_id, "action_id")?;
        let entries = self
            .evidence_repo
            .list(action_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListEvidenceResponse {
            entries: entries.into_iter().map(evidence_to_proto).collect(),
        }))
    }

    // ── File de relecture ──

    async fn add_review(
        &self,
        request: Request<proto::AddReviewRequest>,
    ) -> Result<Response<proto::ReviewEntry>, Status> {
        let req = request.into_inner();
        let action_id = parse_uuid_arg(&req.action_id, "action_id")?;
        let r = self
            .review_repo
            .add(
                action_id,
                &req.guild_id,
                &req.added_by,
                &req.added_by_name,
                req.reason.as_deref(),
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(review_to_proto(r)))
    }

    async fn list_pending_reviews(
        &self,
        request: Request<proto::ListPendingReviewsRequest>,
    ) -> Result<Response<proto::ListPendingReviewsResponse>, Status> {
        let req = request.into_inner();
        let entries = self
            .review_repo
            .list_pending(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListPendingReviewsResponse {
            entries: entries.into_iter().map(review_to_proto).collect(),
        }))
    }

    async fn resolve_review(
        &self,
        request: Request<proto::ResolveReviewRequest>,
    ) -> Result<Response<proto::ResolveReviewResponse>, Status> {
        let req = request.into_inner();
        let review_id = parse_uuid_arg(&req.review_id, "review_id")?;
        let resolved = self
            .review_repo
            .resolve(
                review_id,
                &req.reviewer_id,
                &req.reviewer_name,
                req.notes.as_deref(),
                &req.status,
            )
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ResolveReviewResponse { resolved }))
    }

    // ── Mode apprenti ──

    async fn resolve_pending_action(
        &self,
        request: Request<proto::ResolvePendingActionRequest>,
    ) -> Result<Response<proto::ResolvePendingActionResponse>, Status> {
        let req = request.into_inner();
        let id = parse_uuid_arg(&req.action_id, "action_id")?;
        self.pending_action_repo
            .resolve(id, &req.status, &req.reviewed_by)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ResolvePendingActionResponse {}))
    }

    async fn count_user_infractions(
        &self,
        request: Request<proto::CountUserInfractionsRequest>,
    ) -> Result<Response<proto::UserInfractionCounts>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() || req.user_id.is_empty() {
            return Err(Status::invalid_argument("guild_id et user_id requis"));
        }
        let c = self
            .infractions_uc
            .count_user_infractions(&req.guild_id, &req.user_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::UserInfractionCounts {
            warns: c.warns,
            deletes: c.deletes,
            mutes: c.mutes,
            bans: c.bans,
            total: c.total,
        }))
    }
}

/// Parse un UUID d'argument, en nommant le champ fautif dans l'erreur.
fn parse_uuid_arg(raw: &str, champ: &str) -> Result<uuid::Uuid, Status> {
    raw.parse()
        .map_err(|_| Status::invalid_argument(format!("{champ} doit etre un UUID")))
}

fn evidence_to_proto(
    e: sentinel_core::ports::outbound::moderation::evidence_repository::EvidenceEntry,
) -> proto::EvidenceEntry {
    proto::EvidenceEntry {
        id: e.id.to_string(),
        action_id: e.action_id.to_string(),
        url: e.url,
        description: e.description,
        uploaded_by: e.uploaded_by,
        uploaded_by_name: e.uploaded_by_name,
        uploaded_at: e.uploaded_at.to_rfc3339(),
    }
}

fn review_to_proto(
    r: sentinel_core::ports::outbound::moderation::review_repository::ReviewEntry,
) -> proto::ReviewEntry {
    proto::ReviewEntry {
        id: r.id.to_string(),
        action_id: r.action_id.to_string(),
        guild_id: r.guild_id.into(),
        added_by: r.added_by,
        added_by_name: r.added_by_name,
        reason: r.reason,
        status: r.status,
        reviewer_id: r.reviewer_id,
        reviewer_name: r.reviewer_name,
        reviewer_notes: r.reviewer_notes,
        added_at: r.added_at.to_rfc3339(),
        resolved_at: r.resolved_at.map(|d| d.to_rfc3339()),
        action_type: r.action_type,
        target_name: r.target_name,
        action_reason: r.action_reason,
    }
}

fn moderation_action_to_proto(a: ModerationAction) -> proto::ModerationAction {
    proto::ModerationAction {
        id: a.id.to_string(),
        guild_id: a.guild_id.into(),
        channel_id: a.channel_id.into(),
        moderator_id: a.moderator_id,
        moderator_name: a.moderator_name,
        target_id: a.target_id,
        target_name: a.target_name,
        action_type: a.action_type,
        reason: a.reason,
        gravity: a.gravity.map(|g| g.as_str().to_string()),
        duration: a.duration,
        created_at: a.created_at.to_rfc3339(),
        // Renseignes uniquement en reponse de LogAction (overrides plus bas).
        strikes_count: None,
        escalation_action: None,
        escalation_duration: None,
    }
}

fn user_history_to_proto(h: UserModerationHistory) -> proto::UserHistory {
    proto::UserHistory {
        target_id: h.target_id,
        target_name: h.target_name,
        total_warns: h.total_warns,
        total_mutes: h.total_mutes,
        total_bans: h.total_bans,
        actions: h
            .actions
            .into_iter()
            .map(moderation_action_to_proto)
            .collect(),
    }
}

#[cfg(test)]
#[path = "tests/actions.rs"]
mod tests;
