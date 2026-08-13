use async_trait::async_trait;

use crate::sentinel::domain::entities::ai::dataset::DatasetPage;
use crate::sentinel::domain::errors::DomainError;

/// Filtres bruts de listing tels que recus du handler (avant bornage).
pub struct ListDatasetQuery {
    pub guild_id: String,
    pub channel_id: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub min_length: Option<i32>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Commande de suppression en masse (ids bruts, valides par le use case).
pub struct BulkDeleteCommand {
    pub guild_id: String,
    pub ids: Vec<String>,
}

#[async_trait]
pub trait ManageDatasetUseCase: Send + Sync {
    /// Ingestion d'un message collecte : valide (champs non vides) puis
    /// delegue au repository. Chemin chaud, best-effort cote appelant.
    async fn collect_message(
        &self,
        msg: crate::sentinel::ports::outbound::ai::dataset_repository::NewDatasetMessage,
    ) -> Result<(), DomainError>;
    /// Liste paginee des messages du dataset : borne les parametres puis
    /// delegue au repository.
    async fn list_messages(&self, query: ListDatasetQuery) -> Result<DatasetPage, DomainError>;
    /// Supprime les messages exportes : valide les ids (UUID, plafond) puis
    /// delegue. Renvoie le nombre de lignes effacees.
    async fn bulk_delete(&self, cmd: BulkDeleteCommand) -> Result<i64, DomainError>;
}
