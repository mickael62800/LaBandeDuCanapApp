//! Audit serveur : action admin sur l'infra (vs `audit_logs` = events Discord).
//! Row de la table `server_events`.

use chrono::{DateTime, Utc};

/// Un event serveur (action admin sur l'infra) tel que persiste en BDD.
#[derive(Debug, Clone)]
pub struct ServerEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub actor: Option<String>,
    pub actor_name: Option<String>,
    pub action: String,
    pub target: Option<String>,
    pub severity: String,
    pub details: serde_json::Value,
}

/// Filtres de lecture des events serveur.
#[derive(Debug, Clone)]
pub struct ServerEventFilter {
    /// Prefixe de l'action (`action LIKE prefix || '%'`).
    pub action_prefix: Option<String>,
    /// "info" | "warn" | "critical".
    pub severity: Option<String>,
    /// Nombre max de rows (deja borne par le use case).
    pub limit: i64,
}
