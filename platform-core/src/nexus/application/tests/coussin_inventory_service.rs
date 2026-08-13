use async_trait::async_trait;
use std::sync::{Arc, Mutex};

use crate::nexus::application::coussin_inventory_service::CoussinInventoryService;
use crate::nexus::application::economy_config::EmptyBotConfigRepository;
use crate::nexus::domain::errors::DomainError;
use crate::nexus::ports::inbound::coussin_inventory::CoussinInventoryUseCase;
use crate::nexus::ports::outbound::coussin_inventory_repository::{
    CoussinInventoryRepository, InventoryItem,
};

#[derive(Default)]
struct MockInventoryRepo {
    items: Mutex<Vec<InventoryItem>>,
}

#[async_trait]
impl CoussinInventoryRepository for MockInventoryRepo {
    async fn list(&self, _g: &str, _u: &str) -> Result<Vec<InventoryItem>, DomainError> {
        Ok(self.items.lock().unwrap().clone())
    }
    async fn buy(&self, _g: &str, _u: &str, key: &str, _price: i64) -> Result<i64, DomainError> {
        let mut list = self.items.lock().unwrap();
        if let Some(it) = list.iter_mut().find(|i| i.item_key == key) {
            it.quantity += 1;
        } else {
            list.push(InventoryItem {
                item_key: key.into(),
                quantity: 1,
            });
        }
        Ok(100) // remaining balance
    }
}

#[tokio::test]
async fn test_buy_unknown_item_fails() {
    let service = CoussinInventoryService::new(
        Arc::new(MockInventoryRepo::default()),
        Arc::new(EmptyBotConfigRepository),
    );
    let res = service.buy("g1", "u1", "unknown_item").await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_buy_valid_item_success() {
    let repo = Arc::new(MockInventoryRepo::default());
    let service = CoussinInventoryService::new(repo.clone(), Arc::new(EmptyBotConfigRepository));
    let res = service.buy("g1", "u1", "rage").await;
    assert!(res.is_ok());

    let inv = service.inventory("g1", "u1").await.unwrap();
    assert_eq!(inv.len(), 1);
    assert_eq!(inv[0].item_key, "rage");
    assert_eq!(inv[0].quantity, 1);
}
