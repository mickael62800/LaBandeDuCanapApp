//! Port outbound : plages d'ouverture recurrentes d'un serveur.

use async_trait::async_trait;

use crate::nexus::domain::entities::game::schedule::TimeRange;
use crate::nexus::domain::errors::DomainError;

/// Reglages enregistres pour un serveur.
#[derive(Debug, Clone)]
pub struct StoredSchedule {
    pub server_id: uuid::Uuid,
    pub enabled: bool,
    pub timezone: String,
    pub ranges: Vec<TimeRange>,
    pub warn_minutes: u16,
    /// Dernier preavis envoye, pour ne pas repeter l'annonce a chaque passage.
    pub last_warned_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[async_trait]
pub trait GameScheduleRepository: Send + Sync {
    async fn find(&self, server_id: uuid::Uuid) -> Result<Option<StoredSchedule>, DomainError>;

    /// Tous les horaires ACTIFS, pour le passage periodique. Les serveurs sans
    /// horaire n'ont rien a decider.
    async fn list_enabled(&self) -> Result<Vec<StoredSchedule>, DomainError>;

    async fn upsert(
        &self,
        server_id: uuid::Uuid,
        enabled: bool,
        timezone: &str,
        ranges: &[TimeRange],
        warn_minutes: u16,
        actor: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Note qu'un preavis vient de partir.
    async fn mark_warned(&self, server_id: uuid::Uuid) -> Result<(), DomainError>;

    /// Efface le preavis pour la plage suivante.
    ///
    /// Sans cette remise a zero, un serveur prevenu une fois ne le serait plus
    /// jamais : l'annonce du soir vaudrait pour toutes les soirees suivantes.
    async fn clear_warning(&self, server_id: uuid::Uuid) -> Result<(), DomainError>;
}
