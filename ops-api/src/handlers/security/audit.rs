//! Audit : journal des actions admin, derniers logins, purge des logs.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::{actor_from, record_event};
use crate::{ApiError, AppState};
use ops_core::domain::entities::security_audit::{AuditLogFilter, CleanupOptions};

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
        limit: crate::handlers::normalize_in(q.limit, 100, 1, 500),
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
    let limit = crate::handlers::normalize_in(q.limit, 20, 1, 200);
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

#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub deleted_api_logs: i64,
    pub deleted_audit_logs: i64,
    pub deleted_server_events: i64,
    pub deleted_successful_logins: i64,
    pub deleted_manual_bans: i64,
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

    let actor = actor_from(&headers);
    tracing::info!(
        target: "audit::security",
        actor = actor,
        api_logs = report.deleted_api_logs,
        audit_logs = report.deleted_audit_logs,
        server_events = report.deleted_server_events,
        successful_logins = report.deleted_successful_logins,
        manual_bans = report.deleted_manual_bans,
        days_kept = options.older_than_days,
        "security cleanup executed"
    );
    record_event(
        &state.server_events,
        &actor,
        None,
        "security.cleanup",
        Some(&format!("days={}", options.older_than_days)),
        if options.include_audit_logs {
            "warn"
        } else {
            "info"
        },
        serde_json::json!({
            "deleted_api_logs": report.deleted_api_logs,
            "deleted_audit_logs": report.deleted_audit_logs,
            "deleted_server_events": report.deleted_server_events,
            "deleted_successful_logins": report.deleted_successful_logins,
            "deleted_manual_bans": report.deleted_manual_bans,
            "days_kept": options.older_than_days,
        }),
    )
    .await;

    Ok(Json(CleanupResponse {
        deleted_api_logs: report.deleted_api_logs as i64,
        deleted_audit_logs: report.deleted_audit_logs as i64,
        deleted_server_events: report.deleted_server_events as i64,
        deleted_successful_logins: report.deleted_successful_logins as i64,
        deleted_manual_bans: report.deleted_manual_bans as i64,
        message: format!(
            "{} logs API, {} audit, {} events, {} logins, {} bans manuels supprimes",
            report.deleted_api_logs,
            report.deleted_audit_logs,
            report.deleted_server_events,
            report.deleted_successful_logins,
            report.deleted_manual_bans
        ),
    }))
}
