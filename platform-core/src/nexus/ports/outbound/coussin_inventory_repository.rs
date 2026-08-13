use crate::nexus::domain::errors::DomainError;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub item_key: String,
    pub quantity: i32,
}

#[async_trait]
pub trait CoussinInventoryRepository: Send + Sync {
    async fn list(&self, guild_id: &str, user_id: &str) -> Result<Vec<InventoryItem>, DomainError>;
    async fn buy(
        &self,
        guild_id: &str,
        user_id: &str,
        item_key: &str,
        price: i64,
    ) -> Result<i64, DomainError>;
}
