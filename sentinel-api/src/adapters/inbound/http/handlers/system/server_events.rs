//! Audit serveur : actions admin sur l'infra (vs audit_logs = events Discord).
//!
//! Adaptateur ENTRANT mince : le SQL vit dans `ServerEventRepository`, le bornage
//! des filtres dans `ManageServerEventsUseCase`. Ici : parse -> user -> use case.
//! Helper `record_server_event` : ecriture best-effort (log l'erreur sans bloquer
//! l'action principale de l'appelant).

use std::sync::Arc;

use axum::extract::Query;
use axum::extract::State;
use axum::Extension;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::adapters::inbound::http::errors::ApiError;
use crate::adapters::inbound::http::middleware::superadmin::WebUser;
use crate::bootstrap::state::OpsState;
use sentinel_core::ports::inbound::ops::manage_server_events::ManageServerEventsUseCase;

/// Insere un event serveur via le use case. Best-effort : si echec, on log
/// l'erreur mais on ne bloque pas l'action principale qui appelle ce helper.
///
/// Severities :
/// - "info"     : action normale d'admin (start container, cleanup logs)
/// - "warn"     : action a surveiller (force prune, role grant a un nouveau)
/// - "critical" : action destructive importante (delete volume, prune system)
pub async fn record_server_event(
    uc: &Arc<dyn ManageServerEventsUseCase>,
    actor: &str,
    actor_name: Option<&str>,
    action: &str,
    target: Option<&str>,
    severity: &str,
    details: serde_json::Value,
) {
    if let Err(e) = uc
        .record(actor, actor_name, action, target, severity, details)
        .await
    {
        tracing::warn!(error = %e, action = action, "Echec insert server_events");
    }
}

// ── Endpoint : lire les events ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ServerEventsQuery {
    pub action_prefix: Option<String>,
    pub severity: Option<String>, // "info" | "warn" | "critical"
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ServerEventDto {
    pub id: String,
    pub timestamp: String,
    pub actor: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub severity: String,
    pub details: serde_json::Value,
}

pub async fn list_server_events(
    State(state): State<OpsState>,
    _user: Option<Extension<WebUser>>,
    Query(q): Query<ServerEventsQuery>,
) -> Result<Json<Vec<ServerEventDto>>, ApiError> {
    let events = state
        .server_events_uc
        .list(q.action_prefix, q.severity, q.limit)
        .await?;

    let out = events
        .into_iter()
        .map(|e| ServerEventDto {
            id: e.id,
            timestamp: e.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            actor: e.actor,
            actor_name: e.actor_name,
            action: e.action,
            target: e.target,
            severity: e.severity,
            details: e.details,
        })
        .collect();
    Ok(Json(out))
}
