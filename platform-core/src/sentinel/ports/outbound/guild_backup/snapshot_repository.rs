//! Port outbound de persistance des sauvegardes de serveur.
//!
//! Le payload persiste est le [`GuildSnapshot`] serialise (JSONB cote
//! Postgres). `list` ne remonte QUE les metadonnees (pas le payload) pour la
//! performance.

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::guild_backup::snapshot::GuildSnapshot;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::guild_backup::manage_snapshots::SnapshotSummary;

#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Insere une nouvelle sauvegarde et renvoie l'id genere.
    async fn insert(&self, snapshot: &GuildSnapshot) -> Result<Uuid, DomainError>;

    /// Liste les resumes (metadonnees seules) d'une guild, du plus recent au
    /// plus ancien.
    async fn list(&self, guild_id: &str) -> Result<Vec<SnapshotSummary>, DomainError>;

    /// Charge le payload complet d'une sauvegarde.
    async fn get(&self, id: Uuid) -> Result<Option<GuildSnapshot>, DomainError>;

    /// Supprime une sauvegarde. Renvoie `true` si une ligne existait.
    async fn delete(&self, id: Uuid) -> Result<bool, DomainError>;

    /// Met a jour le label d'une sauvegarde. Renvoie `true` si une ligne
    /// existait.
    async fn rename(&self, id: Uuid, label: &str) -> Result<bool, DomainError>;

    /// Nombre de sauvegardes d'une guild (borne du quota par guild).
    async fn count(&self, guild_id: &str) -> Result<u32, DomainError>;

    /// Id de la sauvegarde la PLUS ANCIENNE d'une guild (pour l'eviction quand
    /// le quota est atteint). `None` si la guild n'en a aucune.
    async fn oldest_id(&self, guild_id: &str) -> Result<Option<Uuid>, DomainError>;
}
