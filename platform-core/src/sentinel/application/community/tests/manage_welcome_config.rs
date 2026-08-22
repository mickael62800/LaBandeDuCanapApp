use super::*;
use async_trait::async_trait;
use std::sync::Mutex;

use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase;
use crate::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;

#[derive(Default)]
struct MockWelcomeConfigRepo {
    configs: Mutex<Vec<String>>,
}

#[async_trait]
impl WelcomeConfigRepository for MockWelcomeConfigRepo {
    async fn get_config(&self, _guild_id: &str) -> Result<Option<String>, DomainError> {
        Ok(self.configs.lock().unwrap().first().cloned())
    }

    async fn save_config(&self, _guild_id: &str, config: &str) -> Result<(), DomainError> {
        self.configs.lock().unwrap().clear();
        self.configs.lock().unwrap().push(config.to_string());
        Ok(())
    }
}

#[tokio::test]
async fn set_welcome_config() {
    let repo = std::sync::Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo);
    let result = svc.set_config("guild123", "Welcome message").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_welcome_config() {
    let repo = std::sync::Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo);
    svc.set_config("guild123", "Hello").await.unwrap();
    let config = svc.get_config("guild123").await;
    assert!(config.is_ok());
}
