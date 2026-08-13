//! Use case inbound de gestion des sauvegardes de serveur.
//!
//! Chaque appel a `store_snapshot` cree une NOUVELLE version (pas
//! d'idempotence : deux captures identiques donnent deux entrees). La liste
//! ne renvoie que des resumes legers (`SnapshotSummary`) — le payload complet
//! n'est charge que via `get_snapshot` (restauration).

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::guild_backup::snapshot::GuildSnapshot;
use crate::sentinel::domain::errors::DomainError;

/// Identifiant d'une sauvegarde stockee.
pub type SnapshotId = Uuid;

/// Resume leger d'une sauvegarde (liste sans le payload complet).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotSummary {
    pub id: SnapshotId,
    pub guild_id: String,
    pub label: String,
    /// RFC3339.
    pub created_at: String,
    pub created_by: Option<String>,
    pub schema_version: u32,
    /// Nombre de roles captures (affichage liste).
    pub role_count: u32,
    /// Nombre de salons captures (affichage liste).
    pub channel_count: u32,
}

#[async_trait]
pub trait ManageGuildSnapshotsUseCase: Send + Sync {
    /// Stocke une nouvelle sauvegarde (nouvelle version). Renvoie son id.
    /// Utilise le quota de rétention par défaut.
    async fn store_snapshot(&self, snapshot: GuildSnapshot) -> Result<SnapshotId, DomainError>;

    /// Comme `store_snapshot` mais avec un quota de rétention explicite
    /// (nombre max de sauvegardes conservées par serveur, configurable).
    async fn store_snapshot_with_quota(
        &self,
        snapshot: GuildSnapshot,
        quota: u32,
    ) -> Result<SnapshotId, DomainError>;

    /// Liste les sauvegardes d'une guild (resumes, sans payload), du plus
    /// recent au plus ancien.
    async fn list_snapshots(&self, guild_id: &str) -> Result<Vec<SnapshotSummary>, DomainError>;

    /// Charge la sauvegarde complete (pour la restauration).
    async fn get_snapshot(&self, snapshot_id: SnapshotId) -> Result<GuildSnapshot, DomainError>;

    /// Supprime une sauvegarde. Renvoie `true` si une ligne a ete supprimee.
    async fn delete_snapshot(&self, snapshot_id: SnapshotId) -> Result<bool, DomainError>;

    /// Renomme une sauvegarde (label). Renvoie `true` si une ligne a ete mise
    /// a jour. Le label ne doit pas etre vide.
    async fn rename_snapshot(
        &self,
        snapshot_id: SnapshotId,
        label: &str,
    ) -> Result<bool, DomainError>;
}
