use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

use crate::sentinel::domain::errors::DomainError;

#[derive(Debug, Clone)]
pub struct EvidenceEntry {
    pub id: Uuid,
    pub action_id: Uuid,
    pub url: String,
    pub description: Option<String>,
    pub uploaded_by: String,
    pub uploaded_by_name: String,
    pub uploaded_at: DateTime<Utc>,
}

#[async_trait]
pub trait EvidenceRepository: Send + Sync {
    async fn add(
        &self,
        action_id: Uuid,
        url: &str,
        description: Option<&str>,
        uploaded_by: &str,
        uploaded_by_name: &str,
    ) -> Result<EvidenceEntry, DomainError>;
    async fn list(&self, action_id: Uuid) -> Result<Vec<EvidenceEntry>, DomainError>;
}
