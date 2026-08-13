use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::dataset::{DatasetPage, DatasetQuery};
use crate::sentinel::domain::errors::DomainError;

/// Message a ingerer dans le dataset (collecte au fil de l'eau depuis le bot).
pub struct NewDatasetMessage {
    pub guild_id: String,
    pub channel_id: String,
    pub channel_name: Option<String>,
    pub user_id: String,
    pub content: String,
}

/// Adapter sortant du dataset IA : tout le SQL sur `ai_dataset_messages`.
#[async_trait]
pub trait DatasetRepository: Send + Sync {
    /// Insere un message collecte (ingestion best-effort, chemin chaud).
    async fn insert_message(&self, msg: &NewDatasetMessage) -> Result<(), DomainError>;
    /// Liste paginee des messages du dataset selon les filtres, avec le total
    /// (compte hors pagination) pour le meme jeu de filtres.
    async fn list_messages(&self, query: &DatasetQuery) -> Result<DatasetPage, DomainError>;
    /// Supprime en masse les messages du `guild_id` dont l'id est fourni.
    /// Renvoie le nombre de lignes effacees.
    async fn bulk_delete(&self, guild_id: &str, ids: &[Uuid]) -> Result<i64, DomainError>;
}
