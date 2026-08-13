use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::ai::ai_job::{AiJob, NewAiJob};
use crate::sentinel::domain::errors::DomainError;

/// Adapter sortant de la file de jobs IA : tout le SQL sur `ai_jobs`.
#[async_trait]
pub trait AiJobRepository: Send + Sync {
    /// Insere un nouveau job (statut `pending`) et renvoie son id.
    async fn enqueue(&self, job: &NewAiJob) -> Result<Uuid, DomainError>;
    /// Recupere l'etat courant d'un job par id, ou `None` s'il n'existe pas.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<AiJob>, DomainError>;
}
