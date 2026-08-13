//! Client du module audit, entierement en gRPC (`AuditService`).
//!
//! Le dashboard web garde les routes HTTP correspondantes : il filtre par
//! plage de dates, par natures multiples et pagine — le bot ne fait qu'une
//! recherche simple, qu'il affiche dans un embed.

use std::sync::Arc;

use serde::Deserialize;

use crate::shared::grpc_client::{grpc_err_to_string, GrpcCallError, SentinelGrpcClient};
use platform_proto::sentinel::audit::v1 as proto;

/// Rapport hebdomadaire agrege server-side (fenetre 7 jours).
/// Le bot ne fait que rendre l'embed a partir de ces compteurs.
#[derive(Debug, Default, Deserialize)]
pub struct WeeklyReport {
    pub member_joins: u64,
    pub member_leaves: u64,
    pub bans: u64,
    pub messages_deleted: u64,
    pub messages_edited: u64,
    pub role_changes: u64,
    pub channel_changes: u64,
    pub voice_events: u64,
    pub anomalies: u64,
}

/// Evenement d'audit observe par le bot et transmis a l'API.
#[derive(Debug)]
pub struct AuditEvent {
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
}

/// Entree du journal, reduite aux champs affiches par `/audit search`.
///
/// L'API renvoyait auparavant un JSON libre dont seuls ces trois champs
/// etaient lus ; le contrat gRPC les nomme explicitement.
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub event_type: String,
    pub actor_name: Option<String>,
    pub target_name: Option<String>,
}

pub struct ApiClient {
    grpc: Arc<SentinelGrpcClient>,
}

impl ApiClient {
    pub fn new(grpc: Arc<SentinelGrpcClient>) -> Self {
        Self { grpc }
    }

    pub async fn search_audit_logs(
        &self,
        guild_id: &str,
        target_id: Option<&str>,
        event_type: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AuditLogEntry>, String> {
        let req = proto::SearchAuditLogsRequest {
            guild_id: guild_id.to_string(),
            target_id: target_id.map(str::to_string),
            event_type: event_type.map(str::to_string),
            limit,
        };
        let resp = crate::grpc_call!(self.grpc, audit, search_audit_logs, req)?;
        Ok(resp
            .entries
            .into_iter()
            .map(|e| AuditLogEntry {
                event_type: e.event_type,
                actor_name: e.actor_name,
                target_name: e.target_name,
            })
            .collect())
    }

    /// Identifiants des membres surveilles d'un serveur.
    ///
    /// `guild_id` est OBLIGATOIRE cote API : sans lui la liste serait globale
    /// et echapperait au cloisonnement par serveur.
    pub async fn get_all_watched_user_ids(&self, guild_id: &str) -> Result<Vec<String>, String> {
        let req = proto::ListWatchedUserIdsRequest {
            guild_id: guild_id.to_string(),
            limit: 1000,
        };
        let resp = crate::grpc_call!(self.grpc, audit, list_watched_user_ids, req)?;
        Ok(resp.user_ids)
    }

    /// Enregistre un evenement d'activite pour un membre surveille.
    #[allow(clippy::too_many_arguments)]
    pub async fn log_user_activity(
        &self,
        guild_id: &str,
        user_id: &str,
        event_type: &str,
        channel_id: Option<&str>,
        channel_name: Option<&str>,
        content: Option<&str>,
        metadata: serde_json::Value,
    ) -> Result<(), String> {
        let req = proto::LogUserActivityRequest {
            guild_id: guild_id.to_string(),
            user_id: user_id.to_string(),
            event_type: event_type.to_string(),
            channel_id: channel_id.map(str::to_string),
            channel_name: channel_name.map(str::to_string),
            content: content.map(str::to_string),
            metadata_json: metadata.to_string(),
        };
        // Best-effort, comme le `post_fire_and_forget` d'origine : perdre une
        // trace d'activite ne doit pas interrompre le traitement de l'event
        // Discord qui l'a produite.
        let res: Result<(), GrpcCallError> =
            crate::grpc_call!(@raw_unit self.grpc, audit, log_user_activity, req);
        if let Err(e) = res {
            tracing::warn!(error = %e, user_id = %user_id, "trace d'activite perdue (best-effort)");
        }
        Ok(())
    }

    /// Rapport d'activite hebdomadaire agrege server-side.
    pub async fn get_weekly_report(&self, guild_id: &str) -> Result<WeeklyReport, String> {
        let req = proto::GetWeeklyReportRequest {
            guild_id: guild_id.to_string(),
        };
        let r = crate::grpc_call!(self.grpc, audit, get_weekly_report, req)?;
        Ok(WeeklyReport {
            member_joins: r.member_joins,
            member_leaves: r.member_leaves,
            bans: r.bans,
            messages_deleted: r.messages_deleted,
            messages_edited: r.messages_edited,
            role_changes: r.role_changes,
            channel_changes: r.channel_changes,
            voice_events: r.voice_events,
            anomalies: r.anomalies,
        })
    }

    pub async fn send_audit_event(&self, event: &AuditEvent) -> Result<(), String> {
        let req = proto::CreateAuditLogRequest {
            guild_id: event.guild_id.clone(),
            event_type: event.event_type.clone(),
            actor_id: event.actor_id.clone(),
            actor_name: event.actor_name.clone(),
            target_id: event.target_id.clone(),
            target_name: event.target_name.clone(),
            channel_id: event.channel_id.clone(),
            channel_name: event.channel_name.clone(),
            details_json: event.details.to_string(),
        };
        crate::grpc_call!(@unit self.grpc, audit, create_audit_log, req)
    }

    /// Transmet un evenement de moderation ; l'API agrege sur sa fenetre
    /// glissante, decide s'il y a anomalie et renvoie l'alerte le cas echeant.
    ///
    /// La DECISION est server-side : le bot n'affiche l'embed URGENT que si
    /// `alert` est present.
    pub async fn detect_moderation_anomaly(
        &self,
        guild_id: &str,
        category: &str,
        increment: usize,
        window_secs: u64,
        thresholds: &super::anomaly::AnomalyThresholds,
    ) -> Result<Option<super::anomaly::AnomalyAlert>, String> {
        let req = proto::DetectAnomalyRequest {
            guild_id: guild_id.to_string(),
            category: category.to_string(),
            increment: increment as u64,
            window_secs,
            mass_ban: thresholds.mass_ban as u64,
            mass_delete: thresholds.mass_delete as u64,
            mass_role_change: thresholds.mass_role_change as u64,
        };
        let resp = crate::grpc_call!(self.grpc, audit, detect_moderation_anomaly, req)?;
        Ok(resp.alert.map(|a| super::anomaly::AnomalyAlert {
            anomaly_type: a.anomaly_type,
            count: a.count as usize,
            window_secs: a.window_secs,
        }))
    }
}
