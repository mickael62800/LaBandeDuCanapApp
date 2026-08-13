//! Use case de la file de jobs IA : valide le type de job (whitelist) et le
//! guild_id avant d'enqueuer. Le SQL vit dans `AiJobRepository`, le handler
//! HTTP ne fait que parse/map.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::ai_job::{AiJob, NewAiJob};
use crate::sentinel::domain::entities::system::job_whitelists::is_valid_ai_job_type;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::ai::manage_ai_jobs::ManageAiJobsUseCase;
use crate::sentinel::ports::outbound::ai::ai_job_repository::AiJobRepository;

pub struct ManageAiJobsService {
    repo: Arc<dyn AiJobRepository>,
}

impl ManageAiJobsService {
    pub fn new(repo: Arc<dyn AiJobRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ManageAiJobsUseCase for ManageAiJobsService {
    async fn create_job(&self, job: NewAiJob) -> Result<Uuid, DomainError> {
        if !is_valid_ai_job_type(&job.job_type) {
            return Err(DomainError::ValidationError(format!(
                "job_type invalide : '{}', attendu 'analyze_text' ou 'analyze_image'",
                job.job_type
            )));
        }
        crate::sentinel::application::validation::validate_guild_id(&job.guild_id)?;
        self.repo.enqueue(&job).await
    }

    async fn get_job(&self, id: Uuid) -> Result<AiJob, DomainError> {
        self.repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| DomainError::NotFound(format!("ai_job {id}")))
    }
}
