use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::ai_job::{AiJob, NewAiJob};
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageAiJobsUseCase: Send + Sync {
    /// Valide (job_type whitelist, guild_id non vide) puis enqueue le job.
    /// Renvoie l'id du job cree.
    async fn create_job(&self, job: NewAiJob) -> Result<Uuid, DomainError>;
    /// Recupere l'etat courant d'un job. `NotFound` si absent.
    async fn get_job(&self, id: Uuid) -> Result<AiJob, DomainError>;
}
