use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::sentinel::application::community::manage_welcome_config_service::ManageWelcomeConfigService;
use crate::sentinel::domain::entities::community::welcome_config::WelcomeConfig;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::community::manage_welcome_config::ManageWelcomeConfigUseCase;
use crate::sentinel::ports::outbound::community::welcome_config_repository::WelcomeConfigRepository;

#[derive(Default)]
struct MockWelcomeConfigRepo {
    configs: Mutex<Vec<WelcomeConfig>>,
}

#[async_trait]
impl WelcomeConfigRepository for MockWelcomeConfigRepo {
    async fn get_config(&self, guild_id: &str) -> Result<Option<WelcomeConfig>, DomainError> {
        Ok(self.configs.lock().await.iter()
            .find(|c| c.guild_id == guild_id)
            .cloned())
    }

    async fn save_config(&self, config: &WelcomeConfig) -> Result<(), DomainError> {
        let mut configs = self.configs.lock().await;
        configs.retain(|c| c.guild_id != config.guild_id);
        configs.push(config.clone());
        Ok(())
    }

    async fn delete_config(&self, guild_id: &str) -> Result<bool, DomainError> {
        let mut configs = self.configs.lock().await;
        let len_before = configs.len();
        configs.retain(|c| c.guild_id != guild_id);
        Ok(configs.len() < len_before)
    }
}

#[tokio::test]
async fn get_welcome_config_returns_none_when_not_found() {
    let repo = Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo);
    let result = svc.get_config("unknown_guild").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[tokio::test]
async fn save_welcome_config_succeeds() {
    let repo = Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo.clone());
    
    let config = WelcomeConfig {
        guild_id: "g1".into(),
        enabled: true,
        message: "Welcome!".into(),
        channel_id: Some("ch1".into()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    let result = svc.save_config(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_welcome_config_returns_saved_config() {
    let repo = Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo.clone());
    
    let config = WelcomeConfig {
        guild_id: "g1".into(),
        enabled: true,
        message: "Welcome!".into(),
        channel_id: Some("ch1".into()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    svc.save_config(&config).await.unwrap();
    let result = svc.get_config("g1").await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[tokio::test]
async fn delete_welcome_config_succeeds() {
    let repo = Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo.clone());
    
    let config = WelcomeConfig {
        guild_id: "g1".into(),
        enabled: true,
        message: "Welcome!".into(),
        channel_id: Some("ch1".into()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    
    svc.save_config(&config).await.unwrap();
    let result = svc.delete_config("g1").await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[tokio::test]
async fn delete_nonexistent_config_returns_false() {
    let repo = Arc::new(MockWelcomeConfigRepo::default());
    let svc = ManageWelcomeConfigService::new(repo);
    let result = svc.delete_config("unknown").await;
    assert!(result.is_ok());
    assert!(!result.unwrap());
}
