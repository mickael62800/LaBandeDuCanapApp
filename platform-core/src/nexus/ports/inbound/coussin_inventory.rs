use crate::nexus::{
    domain::errors::DomainError, ports::outbound::coussin_inventory_repository::InventoryItem,
};
use async_trait::async_trait;
#[async_trait]
pub trait CoussinInventoryUseCase: Send + Sync {
    async fn inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, DomainError>;
    async fn buy(&self, guild_id: &str, user_id: &str, item_key: &str) -> Result<i64, DomainError>;
}
