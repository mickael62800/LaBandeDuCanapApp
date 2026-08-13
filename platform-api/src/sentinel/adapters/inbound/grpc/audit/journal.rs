//! gRPC handler du journal d'audit (recherche, ingestion, surveillance).
//!
//! Miroir des routes HTTP `/api/audit-logs`, `/api/watched-users`,
//! `/api/user-activity`, `/api/audit-weekly-report` et
//! `/api/moderation-anomaly` appelees par le bot.
//!
//! Le web garde ses routes HTTP : il filtre par plage de dates, par natures
//! multiples, pagine et compte. Rien de tout cela n'est expose ici, parce que
//! le bot ne s'en sert pas.

use std::sync::Arc;

use platform_proto::sentinel::audit::v1 as proto;
use platform_proto::sentinel::audit::v1::audit_service_server::AuditService;
use tonic::Request;
use tonic::Response;
use tonic::Status;

use crate::sentinel::adapters::inbound::grpc::errors::domain_to_status;
use platform_core::sentinel::domain::entities::audit::moderation_anomaly::AnomalyThresholds;
use platform_core::sentinel::domain::entities::audit::user_activity::UserActivity;
use platform_core::sentinel::ports::inbound::audit::detect_moderation_anomaly::{
    DetectAnomalyCommand, DetectModerationAnomalyUseCase,
};
use platform_core::sentinel::ports::inbound::audit::get_weekly_report::GetWeeklyReportUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_audit_logs::{
    AuditLogFilters, CreateAuditLogCommand, ManageAuditLogsUseCase,
};
use platform_core::sentinel::ports::inbound::audit::manage_watched_users::ManageWatchedUsersUseCase;
use platform_core::sentinel::ports::outbound::audit::user_activity_repository::UserActivityRepository;

pub struct AuditGrpc {
    pub audit_logs_uc: Arc<dyn ManageAuditLogsUseCase>,
    pub watched_users_uc: Arc<dyn ManageWatchedUsersUseCase>,
    pub weekly_report_uc: Arc<dyn GetWeeklyReportUseCase>,
    pub detect_anomaly_uc: Arc<dyn DetectModerationAnomalyUseCase>,
    pub user_activity_repo: Arc<dyn UserActivityRepository>,
}

/// Parse une charge JSON libre, en tolerant l'absence de valeur.
///
/// Une chaine vide vaut `null` : le bot n'a pas a fabriquer `"null"` quand il
/// n'a rien a joindre a l'evenement.
fn parse_details(raw: &str) -> Result<serde_json::Value, Status> {
    if raw.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(raw).map_err(|e| Status::invalid_argument(format!("JSON invalide : {e}")))
}

#[tonic::async_trait]
impl AuditService for AuditGrpc {
    async fn search_audit_logs(
        &self,
        request: Request<proto::SearchAuditLogsRequest>,
    ) -> Result<Response<proto::SearchAuditLogsResponse>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        // Borne dure : la commande `/audit search` affiche une liste dans un
        // embed Discord, une limite non bornee ne servirait qu'a charger la
        // base pour un resultat illisible.
        let limit = req.limit.clamp(1, 100) as i64;

        let logs = self
            .audit_logs_uc
            .list(
                Some(&req.guild_id),
                AuditLogFilters {
                    event_type: req.event_type,
                    target_id: req.target_id,
                    limit,
                    ..Default::default()
                },
            )
            .await
            .map_err(domain_to_status)?;

        Ok(Response::new(proto::SearchAuditLogsResponse {
            entries: logs
                .into_iter()
                .map(|l| proto::AuditLogEntry {
                    event_type: l.event_type,
                    actor_name: l.actor_name,
                    target_name: l.target_name,
                })
                .collect(),
        }))
    }

    async fn create_audit_log(
        &self,
        request: Request<proto::CreateAuditLogRequest>,
    ) -> Result<Response<proto::CreateAuditLogResponse>, Status> {
        let req = request.into_inner();
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        let details = parse_details(&req.details_json)?;
        let log = self
            .audit_logs_uc
            .create(CreateAuditLogCommand {
                guild_id: req.guild_id.into(),
                event_type: req.event_type,
                actor_id: req.actor_id,
                actor_name: req.actor_name,
                target_id: req.target_id,
                target_name: req.target_name,
                channel_id: req.channel_id,
                channel_name: req.channel_name,
                details,
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::CreateAuditLogResponse {
            id: log.id.to_string(),
        }))
    }

    async fn list_watched_user_ids(
        &self,
        request: Request<proto::ListWatchedUserIdsRequest>,
    ) -> Result<Response<proto::ListWatchedUserIdsResponse>, Status> {
        let req = request.into_inner();
        // `guild_id` obligatoire : sans lui la liste serait globale et
        // echapperait au cloisonnement par serveur.
        if req.guild_id.is_empty() {
            return Err(Status::invalid_argument("guild_id requis"));
        }
        let limit = req.limit.clamp(1, 1000) as i64;
        let users = self
            .watched_users_uc
            .list_watched_users(Some(&req.guild_id), limit, 0)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::ListWatchedUserIdsResponse {
            user_ids: users.into_iter().map(|u| u.user_id.into()).collect(),
        }))
    }

    async fn log_user_activity(
        &self,
        request: Request<proto::LogUserActivityRequest>,
    ) -> Result<Response<proto::LogUserActivityResponse>, Status> {
        let req = request.into_inner();
        let metadata = parse_details(&req.metadata_json)?;
        let activity = UserActivity {
            id: uuid::Uuid::new_v4(),
            guild_id: req.guild_id.into(),
            user_id: req.user_id.into(),
            event_type: req.event_type,
            channel_id: req.channel_id,
            channel_name: req.channel_name,
            content: req.content,
            metadata,
            created_at: chrono::Utc::now(),
        };
        self.user_activity_repo
            .create(&activity)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::LogUserActivityResponse {}))
    }

    async fn get_weekly_report(
        &self,
        request: Request<proto::GetWeeklyReportRequest>,
    ) -> Result<Response<proto::WeeklyReport>, Status> {
        let req = request.into_inner();
        let r = self
            .weekly_report_uc
            .get(&req.guild_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::WeeklyReport {
            member_joins: r.member_joins,
            member_leaves: r.member_leaves,
            bans: r.bans,
            messages_deleted: r.messages_deleted,
            messages_edited: r.messages_edited,
            role_changes: r.role_changes,
            channel_changes: r.channel_changes,
            voice_events: r.voice_events,
            anomalies: r.anomalies,
        }))
    }

    async fn detect_moderation_anomaly(
        &self,
        request: Request<proto::DetectAnomalyRequest>,
    ) -> Result<Response<proto::DetectAnomalyResponse>, Status> {
        let req = request.into_inner();
        let alert = self
            .detect_anomaly_uc
            .detect(DetectAnomalyCommand {
                guild_id: req.guild_id,
                category: req.category,
                increment: req.increment as usize,
                window_secs: req.window_secs,
                thresholds: AnomalyThresholds {
                    mass_ban: req.mass_ban as usize,
                    mass_delete: req.mass_delete as usize,
                    mass_role_change: req.mass_role_change as usize,
                },
            })
            .await;
        Ok(Response::new(proto::DetectAnomalyResponse {
            alert: alert.map(|a| proto::AnomalyAlert {
                anomaly_type: a.anomaly_type,
                count: a.count as u64,
                window_secs: a.window_secs,
            }),
        }))
    }

    async fn get_activity_by_message(
        &self,
        request: Request<proto::GetActivityByMessageRequest>,
    ) -> Result<Response<proto::GetActivityByMessageResponse>, Status> {
        let req = request.into_inner();
        let activity = self
            .user_activity_repo
            .find_by_message_id(&req.guild_id, &req.message_id)
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::GetActivityByMessageResponse {
            content: activity.and_then(|a| a.content),
        }))
    }

    async fn record_name_history(
        &self,
        request: Request<proto::RecordNameHistoryRequest>,
    ) -> Result<Response<proto::RecordNameHistoryResponse>, Status> {
        use platform_core::sentinel::domain::entities::audit::audit_log::AUDIT_EVENT_MEMBER_NICKNAME_HISTORY;
        let req = request.into_inner();
        // Best-effort (comme le handler HTTP) : le mapping event_type/details
        // reste server-side ; le bot ne fournit que les faits.
        self.audit_logs_uc
            .create(CreateAuditLogCommand {
                guild_id: req.guild_id.into(),
                event_type: AUDIT_EVENT_MEMBER_NICKNAME_HISTORY.into(),
                actor_id: None,
                actor_name: None,
                target_id: Some(req.user_id),
                target_name: Some(req.new_name.clone()),
                channel_id: None,
                channel_name: None,
                details: serde_json::json!({
                    "old_name": req.old_name,
                    "new_name": req.new_name,
                }),
            })
            .await
            .map_err(domain_to_status)?;
        Ok(Response::new(proto::RecordNameHistoryResponse {}))
    }
}
