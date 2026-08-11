//! Entites du panel securite cote "audit & maintenance" : journal d'audit
//! admin (`audit_logs`), derniers logins OAuth (`successful_logins`) et purge
//! des logs de securite.

use chrono::{DateTime, Utc};

/// Une entree du journal d'audit admin.
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    pub id: String,
    pub guild_id: String,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Filtre de lecture du journal d'audit.
#[derive(Debug, Clone)]
pub struct AuditLogFilter {
    pub guild_id: Option<String>,
    /// Prefixe d'event_type (ex: "docker." ou "rbac.").
    pub event_type_prefix: Option<String>,
    pub limit: i64,
}

/// Un login OAuth Discord reussi.
#[derive(Debug, Clone)]
pub struct SuccessfulLogin {
    pub timestamp: DateTime<Utc>,
    pub discord_user_id: String,
    pub username: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
}

/// Options de purge des logs de securite (operation destructive).
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    /// Nb de jours a garder. 0 = tout supprimer.
    pub older_than_days: i64,
    pub include_api_logs: bool,
    pub include_audit_logs: bool,
    pub include_server_events: bool,
    pub include_successful_logins: bool,
    pub include_manual_bans: bool,
}

/// Sort d'une cible de purge, expose tel quel a l'operateur.
///
/// Distinguer `Skipped` (non demandee) de `Deleted(0)` (demandee, rien a
/// supprimer) de `Failed` (demandee, mais echouee) : un compteur nul seul ne
/// disait pas laquelle des trois s'etait produite.
#[derive(Debug, Clone)]
pub enum CleanupTargetStatus {
    /// Cible non demandee (option a `false`).
    Skipped,
    /// Purge appliquee — et commitee, pour les tables locales.
    Deleted(u64),
    /// Purge demandee mais echouee (ex. purge distante de l'identite).
    Failed(String),
}

impl CleanupTargetStatus {
    /// Nombre reellement supprime (0 si `Skipped`/`Failed`).
    pub fn deleted(&self) -> u64 {
        match self {
            Self::Deleted(n) => *n,
            _ => 0,
        }
    }

    /// `true` si la cible n'est pas en echec (appliquee ou ignoree).
    pub fn is_ok(&self) -> bool {
        !matches!(self, Self::Failed(_))
    }
}

/// Compte-rendu d'une purge, cible par cible.
#[derive(Debug, Clone)]
pub struct CleanupReport {
    pub api_logs: CleanupTargetStatus,
    pub audit_logs: CleanupTargetStatus,
    pub server_events: CleanupTargetStatus,
    pub successful_logins: CleanupTargetStatus,
    pub manual_bans: CleanupTargetStatus,
}
