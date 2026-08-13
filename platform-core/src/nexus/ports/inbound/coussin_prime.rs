use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;
#[async_trait]
pub trait CoussinPrimeUseCase: Send + Sync {
    async fn place(
        &self,
        guild_id: &str,
        target_id: &str,
        target_name: &str,
        placer_id: &str,
        placer_name: &str,
        amount: i64,
    ) -> Result<(), DomainError>;
}
