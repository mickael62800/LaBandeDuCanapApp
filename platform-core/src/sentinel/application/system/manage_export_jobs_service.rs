//! Use case de la file d'export : delegue l'enqueue et la lecture du statut au
//! `ExportJobRepository`. Pure orchestration — pas d'infra. Le handler HTTP ne
//! fait que parser/RBAC/valider ; le SQL vit dans l'adapter Postgres.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::system::manage_export_jobs::ManageExportJobsUseCase;
use crate::sentinel::ports::outbound::system::export_job_repository::{
    ExportJobRecord, ExportJobRepository, NewExportJob,
};

pub struct ManageExportJobsService {
    repo: Arc<dyn ExportJobRepository>,
}

impl ManageExportJobsService {
    pub fn new(repo: Arc<dyn ExportJobRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageExportJobsUseCase for ManageExportJobsService {
    async fn enqueue(&self, job: NewExportJob) -> Result<Uuid, DomainError> {
        self.repo.enqueue(&job).await
    }

    async fn get(&self, id: Uuid) -> Result<Option<ExportJobRecord>, DomainError> {
        self.repo.find(id).await
    }
}

#[cfg(test)]
#[path = "tests/manage_export_jobs.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/manage_export_jobs.rs"]
mod tests;
