//! Port outbound : archives des mondes de serveurs de jeu.
//!
//! La table `game_backups` existe depuis la migration 007 sans qu'aucun code ne
//! l'alimente. Ce port lui donne enfin un producteur : le redemarrage programme
//! y consigne chaque archive, ce qui permettra a l'interface de lister les
//! sauvegardes disponibles pour un serveur.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::nexus::domain::errors::DomainError;

#[async_trait]
pub trait GameBackupRepository: Send + Sync {
    /// Date de la derniere archive AUTOMATIQUE de ce serveur.
    ///
    /// Sert a espacer les archives : sans elle, une permanence qui redemarre
    /// toutes les trois heures produirait huit copies quasi identiques par jour.
    /// Les archives manuelles sont ignorees — un exploitant qui en declenche une
    /// ne doit pas priver le serveur de sa sauvegarde automatique du lendemain.
    async fn last_auto_backup_at(
        &self,
        server_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, DomainError>;

    /// Consigne une archive ecrite sur le disque.
    async fn record(
        &self,
        server_id: Uuid,
        file_path: &str,
        size_bytes: i64,
        backup_type: &str,
    ) -> Result<(), DomainError>;
}
