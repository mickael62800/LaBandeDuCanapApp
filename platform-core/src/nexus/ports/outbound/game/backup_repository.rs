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

    /// Archives d'un serveur, la plus recente en tete.
    ///
    /// La table avait un producteur mais aucun lecteur : une sauvegarde
    /// declenchee a la main ne se voyait nulle part, et il fallait ouvrir un
    /// terminal sur l'hote pour savoir si elle avait eu lieu.
    async fn list_for_server(
        &self,
        server_id: Uuid,
        limite: i64,
    ) -> Result<Vec<GameBackup>, DomainError>;

    /// Consigne une archive ecrite sur le disque.
    async fn record(
        &self,
        server_id: Uuid,
        file_path: &str,
        size_bytes: i64,
        backup_type: &str,
    ) -> Result<(), DomainError>;
}

/// Une archive telle que l'interface la montre.
#[derive(Debug, Clone)]
pub struct GameBackup {
    pub id: Uuid,
    pub file_path: String,
    pub size_bytes: i64,
    /// `auto` (redemarrage ou fermeture de plage) ou `manual` (bouton).
    pub backup_type: String,
    pub created_at: DateTime<Utc>,
}
