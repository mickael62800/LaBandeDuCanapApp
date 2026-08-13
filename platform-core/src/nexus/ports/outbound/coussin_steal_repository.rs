use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;
#[async_trait]
pub trait CoussinStealRepository: Send + Sync {
    async fn balances(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
    ) -> Result<(i64, i64), DomainError>;
    async fn transfer(
        &self,
        guild_id: &str,
        thief_id: &str,
        victim_id: &str,
        amount: i64,
        success: bool,
        cooldown_minutes: i64,
    ) -> Result<(), DomainError>;
}
