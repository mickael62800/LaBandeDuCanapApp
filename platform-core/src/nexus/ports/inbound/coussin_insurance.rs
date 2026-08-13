use crate::nexus::{
    domain::errors::DomainError, ports::outbound::coussin_insurance_repository::CoussinInsurance,
};
use async_trait::async_trait;
#[async_trait]
pub trait CoussinInsuranceUseCase: Send + Sync {
    async fn buy(&self, guild_id: &str, user_id: &str) -> Result<CoussinInsurance, DomainError>;
    async fn active(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<CoussinInsurance>, DomainError>;
}
