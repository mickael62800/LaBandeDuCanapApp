use crate::nexus::{
    application::economy_config::load_coussin,
    domain::{entities::coussin_shop::item, errors::DomainError},
    ports::{
        inbound::coussin_inventory::CoussinInventoryUseCase,
        outbound::{
            coussin_inventory_repository::{CoussinInventoryRepository, InventoryItem},
            system::bot_config_repository::BotConfigRepository,
        },
    },
};
use async_trait::async_trait;
use std::sync::Arc;
pub struct CoussinInventoryService {
    repo: Arc<dyn CoussinInventoryRepository>,
    config_repo: Arc<dyn BotConfigRepository>,
}
impl CoussinInventoryService {
    pub fn new(
        repo: Arc<dyn CoussinInventoryRepository>,
        config_repo: Arc<dyn BotConfigRepository>,
    ) -> Self {
        Self { repo, config_repo }
    }
}
#[async_trait]
impl CoussinInventoryUseCase for CoussinInventoryService {
    async fn inventory(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<InventoryItem>, DomainError> {
        self.repo.list(guild_id, user_id).await
    }
    /// Le coffre se ferme avec le jeu. La lecture de l'inventaire, elle,
    /// reste ouverte : ce qu'on possede deja ne disparait pas.
    async fn buy(&self, guild_id: &str, user_id: &str, item_key: &str) -> Result<i64, DomainError> {
        let cfg = load_coussin(&self.config_repo, guild_id).await?;
        cfg.ensure_enabled()?;
        let item = item(item_key).ok_or_else(|| DomainError::Validation("objet inconnu".into()))?;
        self.repo
            .buy(
                guild_id,
                user_id,
                item.key,
                cfg.shop_price(item.key, item.price),
            )
            .await
    }
}

#[cfg(test)]
#[path = "tests/coussin_inventory_service.rs"]
mod tests;
