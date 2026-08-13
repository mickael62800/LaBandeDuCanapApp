use async_trait::async_trait;

use crate::atrium::domain::{ServerSummaryReply, ServerSummaryRequest, WelcomeError};

#[async_trait]
pub trait GenerateServerSummaryUseCase: Send + Sync {
    async fn generate_summary(
        &self,
        request: ServerSummaryRequest,
    ) -> Result<ServerSummaryReply, WelcomeError>;
}
