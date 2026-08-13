//! Audit : journal des actions admin, derniers logins, purge des logs.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::{actor_from, record_event};
use crate::ops::{ApiError, AppState};
use platform_core::ops::domain::entities::security_audit::{AuditLogFilter, CleanupOptions};

// ── Audit log admin (actions sensibles) ─────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub guild_id: Option<String>,
    pub event_type_prefix: Option<String>, // ex: "docker." ou "user."
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditEntry {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: String,
}

pub async fn audit_logs(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>, ApiError> {
    let filter = AuditLogFilter {
        guild_id: q.guild_id,
        event_type_prefix: q.event_type_prefix,
        limit: crate::ops::handlers::normalize_in(q.limit, 100, 1, 500),
    };
    let rows = state.security_audit_uc.audit_logs(filter).await?;
    Ok(Json(
        rows.into_iter()
            .map(|e| AuditEntry {
                id: e.id,
                guild_id: e.guild_id,
                event_type: e.event_type,
                actor_id: e.actor_id,
                actor_name: e.actor_name,
                target_id: e.target_id,
                target_name: e.target_name,
                details: e.details,
                created_at: e.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
            .collect(),
    ))
}

// ── Last successful logins ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SuccessfulLoginEntry {
    pub timestamp: String,
    pub discord_user_id: String,
    pub username: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<i64>,
}

pub async fn last_successful_logins(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> Result<Json<Vec<SuccessfulLoginEntry>>, ApiError> {
    let limit = crate::ops::handlers::normalize_in(q.limit, 20, 1, 200);
    let rows = state.security_audit_uc.recent_logins(limit).await?;
    Ok(Json(
        rows.into_iter()
            .map(|l| SuccessfulLoginEntry {
                timestamp: l.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                discord_user_id: l.discord_user_id,
                username: l.username,
                client_ip: l.client_ip,
                user_agent: l.user_agent,
            })
            .collect(),
    ))
}

// ── Cleanup : purge des logs de securite ────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CleanupQuery {
    /// Nb de jours a garder. 0 = tout supprimer. Defaut 0.
    #[serde(default)]
    pub older_than_days: Option<i64>,
    /// True = purger les logs API (Top IPs, auth failures). Defaut true.
    #[serde(default)]
    pub include_api_logs: Option<bool>,
    /// True = purger aussi audit_logs (events Discord). Defaut false.
    #[serde(default)]
    pub include_audit_logs: Option<bool>,
    /// Purge `server_events` (audit infra : ban-ip, docker, user).
    #[serde(default)]
    pub include_server_events: Option<bool>,
    /// Purge `successful_logins` (derniers logins OAuth Discord).
    #[serde(default)]
    pub include_successful_logins: Option<bool>,
    /// Purge `manual_ip_bans` (historique des bans, incl. ceux deja leves).
    #[serde(default)]
    pub include_manual_bans: Option<bool>,
}

/// Sort d'une cible : `status` = "deleted" | "skipped" | "failed".
#[derive(Debug, Serialize)]
pub struct CleanupTargetDto {
    pub status: &'static str,
    pub deleted: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl From<&platform_core::ops::domain::entities::security_audit::CleanupTargetStatus>
    for CleanupTargetDto
{
    fn from(
        status: &platform_core::ops::domain::entities::security_audit::CleanupTargetStatus,
    ) -> Self {
        use platform_core::ops::domain::entities::security_audit::CleanupTargetStatus as St;
        match status {
            St::Deleted(n) => Self {
                status: "deleted",
                deleted: *n,
                error: None,
            },
            St::Skipped => Self {
                status: "skipped",
                deleted: 0,
                error: None,
            },
            St::Failed(reason) => Self {
                status: "failed",
                deleted: 0,
                error: Some(reason.clone()),
            },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub api_logs: CleanupTargetDto,
    pub audit_logs: CleanupTargetDto,
    pub server_events: CleanupTargetDto,
    pub successful_logins: CleanupTargetDto,
    pub manual_bans: CleanupTargetDto,
    /// `false` si au moins une cible demandee a echoue.
    pub all_succeeded: bool,
    pub message: String,
}

/// DELETE /api/security/cleanup
/// Supprime les entrees de logs (table `logs` cat='api') et optionnellement
/// `audit_logs`. Gate superadmin uniquement (operation destructive).
pub async fn cleanup_security_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<CleanupQuery>,
) -> Result<Json<CleanupResponse>, ApiError> {
    // Endpoint cross-guild ultra-destructif (peut DELETE FROM audit_logs
    // global). L'acces est deja garde par la passerelle nginx, qui n'admet
    // que les superadmins : ops-api n'a pas de notion d'utilisateur a
    // reverifier.

    let options = CleanupOptions {
        older_than_days: q.older_than_days.unwrap_or(0).max(0),
        include_api_logs: q.include_api_logs.unwrap_or(true),
        include_audit_logs: q.include_audit_logs.unwrap_or(false),
        include_server_events: q.include_server_events.unwrap_or(false),
        include_successful_logins: q.include_successful_logins.unwrap_or(false),
        include_manual_bans: q.include_manual_bans.unwrap_or(false),
    };

    let report = state.security_audit_uc.cleanup(options.clone()).await?;

    let all_succeeded = report.api_logs.is_ok()
        && report.audit_logs.is_ok()
        && report.server_events.is_ok()
        && report.successful_logins.is_ok()
        && report.manual_bans.is_ok();

    let actor = actor_from(&headers);
    tracing::info!(
        target: "audit::security",
        actor = actor,
        api_logs = report.api_logs.deleted(),
        audit_logs = report.audit_logs.deleted(),
        server_events = report.server_events.deleted(),
        successful_logins = report.successful_logins.deleted(),
        manual_bans = report.manual_bans.deleted(),
        all_succeeded,
        days_kept = options.older_than_days,
        "security cleanup executed"
    );
    record_event(
        &state.server_events,
        &actor,
        None,
        "security.cleanup",
        Some(&format!("days={}", options.older_than_days)),
        if !all_succeeded || options.include_audit_logs {
            "warn"
        } else {
            "info"
        },
        serde_json::json!({
            "deleted_api_logs": report.api_logs.deleted(),
            "deleted_audit_logs": report.audit_logs.deleted(),
            "deleted_server_events": report.server_events.deleted(),
            "deleted_successful_logins": report.successful_logins.deleted(),
            "deleted_manual_bans": report.manual_bans.deleted(),
            "all_succeeded": all_succeeded,
            "days_kept": options.older_than_days,
        }),
    )
    .await;

    let message = format!(
        "{} logs API, {} audit, {} events, {} logins, {} bans manuels supprimes",
        report.api_logs.deleted(),
        report.audit_logs.deleted(),
        report.server_events.deleted(),
        report.successful_logins.deleted(),
        report.manual_bans.deleted()
    );

    Ok(Json(CleanupResponse {
        api_logs: (&report.api_logs).into(),
        audit_logs: (&report.audit_logs).into(),
        server_events: (&report.server_events).into(),
        successful_logins: (&report.successful_logins).into(),
        manual_bans: (&report.manual_bans).into(),
        all_succeeded,
        message,
    }))
}
