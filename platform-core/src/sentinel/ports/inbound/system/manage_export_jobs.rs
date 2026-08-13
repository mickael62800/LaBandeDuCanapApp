//! Port inbound : gestion de la file d'attente des jobs d'export.
//! Le handler HTTP ne fait que parser/RBAC/valider/mapper ; l'enqueue et la
//! lecture du statut passent par ce use case, le SQL dans `ExportJobRepository`.

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::outbound::system::export_job_repository::{
    ExportJobRecord, NewExportJob,
};

#[async_trait]
pub trait ManageExportJobsUseCase: Send + Sync {
    /// Enfile un job d'export et retourne son id.
    async fn enqueue(&self, job: NewExportJob) -> Result<Uuid, DomainError>;

    /// Lit l'etat d'un job d'export (None si inexistant).
    async fn get(&self, id: Uuid) -> Result<Option<ExportJobRecord>, DomainError>;
}
