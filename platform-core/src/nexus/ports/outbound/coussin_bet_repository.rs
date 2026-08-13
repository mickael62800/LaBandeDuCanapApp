use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;
#[async_trait]
pub trait CoussinBetRepository: Send + Sync {
    async fn place(
        &self,
        guild_id: &str,
        combat_id: uuid::Uuid,
        bettor_id: &str,
        bettor_name: &str,
        backed_id: &str,
        amount: i64,
    ) -> Result<(), DomainError>;
}
