//! Port outbound : pilotage d'un serveur dans le temps.
//!
//! Deux systemes exclusifs vivent dans cette table : les plages d'ouverture et
//! la permanence avec redemarrages periodiques. La colonne `mode` tranche —
//! voir `domain::entities::game::schedule`.

use async_trait::async_trait;

use crate::nexus::domain::entities::game::schedule::{ScheduleMode, TimeRange};
use crate::nexus::domain::errors::DomainError;

/// Reglages enregistres pour un serveur.
#[derive(Debug, Clone)]
pub struct StoredSchedule {
    pub server_id: uuid::Uuid,
    pub enabled: bool,
    /// Lequel des deux systemes pilote ce serveur.
    pub mode: ScheduleMode,
    pub timezone: String,
    pub ranges: Vec<TimeRange>,
    pub warn_minutes: u16,
    /// Dernier preavis envoye, pour ne pas repeter l'annonce a chaque passage.
    pub last_warned_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Mode permanence : heures entre deux redemarrages.
    pub restart_interval_hours: Option<u8>,
    /// Minute de l'heure a laquelle tombent les creneaux.
    pub restart_anchor_minute: u8,
    /// Dernier redemarrage programme execute.
    pub last_restart_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Derniere annonce a une minute. Distincte de `last_warned_at` : les deux
    /// portent sur le meme creneau et ne doivent pas s'annuler l'une l'autre.
    pub last_final_warned_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Ce qu'un administrateur enregistre depuis le tableau de bord.
#[derive(Debug, Clone)]
pub struct ScheduleSettings {
    pub enabled: bool,
    pub mode: ScheduleMode,
    pub timezone: String,
    pub ranges: Vec<TimeRange>,
    pub warn_minutes: u16,
    pub restart_interval_hours: Option<u8>,
    pub restart_anchor_minute: u8,
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
        settings: &ScheduleSettings,
        actor: Option<&str>,
    ) -> Result<(), DomainError>;

    /// Note qu'un preavis vient de partir.
    async fn mark_warned(&self, server_id: uuid::Uuid) -> Result<(), DomainError>;

    /// Note que l'annonce finale vient de partir.
    async fn mark_final_warned(&self, server_id: uuid::Uuid) -> Result<(), DomainError>;

    /// Note qu'un redemarrage programme vient d'etre execute, et efface les
    /// deux marqueurs d'annonce pour que le creneau suivant soit annonce.
    async fn mark_restarted(&self, server_id: uuid::Uuid) -> Result<(), DomainError>;

    /// Efface le preavis pour la plage suivante.
    ///
    /// Sans cette remise a zero, un serveur prevenu une fois ne le serait plus
    /// jamais : l'annonce du soir vaudrait pour toutes les soirees suivantes.
    async fn clear_warning(&self, server_id: uuid::Uuid) -> Result<(), DomainError>;
}
