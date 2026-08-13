use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::audit_log::AuditLog;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::errors::DomainError;

pub struct CreateAuditLogCommand {
    pub guild_id: GuildId,
    pub event_type: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub channel_id: Option<String>,
    pub channel_name: Option<String>,
    pub details: serde_json::Value,
}

pub struct AuditLogFilters {
    pub event_type: Option<String>,
    /// Selection multiple. Complete `event_type` (garde pour compat) : le
    /// journal web filtre par NATURE d'evenement (vocal, membres, messages),
    /// ce qui recouvre plusieurs types a la fois.
    pub event_types: Vec<String>,
    pub actor_id: Option<String>,
    pub target_id: Option<String>,
    /// Bornes temporelles (incluses). Indispensables des lors que le web
    /// remplace les salons Discord : sans elles, impossible de remonter a une
    /// date precise autrement qu'en paginant a l'aveugle.
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    /// Recherche libre sur les noms d'acteur/cible/salon.
    pub search: Option<String>,
    pub limit: i64,
    pub offset: i64,
}

impl Default for AuditLogFilters {
    fn default() -> Self {
        Self {
            event_type: None,
            event_types: Vec::new(),
            actor_id: None,
            target_id: None,
            from: None,
            to: None,
            search: None,
            limit: 100,
            offset: 0,
        }
    }
}

#[async_trait]
pub trait ManageAuditLogsUseCase: Send + Sync {
    async fn create(&self, command: CreateAuditLogCommand) -> Result<AuditLog, DomainError>;
    async fn list(
        &self,
        guild_id: Option<&str>,
        filters: AuditLogFilters,
    ) -> Result<Vec<AuditLog>, DomainError>;
    /// Nombre total d'entrees correspondant aux filtres (hors limit/offset).
    /// Permet une pagination serveur reelle cote web.
    async fn count(
        &self,
        guild_id: Option<&str>,
        filters: &AuditLogFilters,
    ) -> Result<i64, DomainError>;

    async fn delete_older_than_days(&self, guild_id: &str, days: i32) -> Result<u64, DomainError>;

    /// Timeline d'un salon vocal (events voice, ordre ASC). Default : vide
    /// (pour les stubs de test).
    async fn list_voice_channel_events(
        &self,
        _channel_id: &str,
        _limit: i64,
    ) -> Result<Vec<AuditLog>, DomainError> {
        Ok(vec![])
    }
}
