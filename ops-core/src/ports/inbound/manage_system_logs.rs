//! Port inbound : lecture/purge des logs systeme (table `logs`).
//!
//! Alimente la page "Logs systeme" du panneau web via l'endpoint
//! `GET /api/logs` (source Postgres) et `DELETE /api/logs/{category}`.
//! Le SQL vit dans `LogRepository` ; ce use case porte le clamp de limite et
//! la garde metier "categorie discord non purgeable".

use async_trait::async_trait;

use crate::domain::entities::log_entry::LogEntry;
use crate::domain::errors::DomainError;

/// Filtres de lecture des logs systeme. `guild_id` est traduit en clause SQL
/// (`server = ...`) par l'adapter — plus de filtrage post-fetch en Rust.
pub struct SystemLogFilters {
    pub category: Option<String>,
    pub level: Option<String>,
    pub guild_id: Option<String>,
    pub limit: i64,
}

#[async_trait]
pub trait ManageSystemLogsUseCase: Send + Sync {
    /// Logs recents filtres (category / level / guild), tries par timestamp DESC.
    async fn list_logs(&self, filters: SystemLogFilters) -> Result<Vec<LogEntry>, DomainError>;

    /// Purge Postgres d'une categorie. `ValidationError` pour `discord`
    /// (categorie protegee). Retourne le nombre de lignes supprimees.
    async fn purge_category(&self, category: &str) -> Result<u64, DomainError>;
}
