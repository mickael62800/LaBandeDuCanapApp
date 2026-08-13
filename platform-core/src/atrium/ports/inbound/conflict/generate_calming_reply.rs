use async_trait::async_trait;

use crate::atrium::domain::{CalmingError, CalmingReply, CalmingRequest};

/// Génère le rappel apaisant à publier dans un salon en tension.
#[async_trait]
pub trait GenerateCalmingReplyUseCase: Send + Sync {
    async fn reply(&self, request: CalmingRequest) -> Result<CalmingReply, CalmingError>;
}
