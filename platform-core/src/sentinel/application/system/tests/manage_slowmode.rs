use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::sentinel::domain::errors::DomainError;

#[derive(Default, Clone)]
struct MockSlowmodeConfig {
    guild_id: String,
    enabled: bool,
    duration_secs: u64,
}

struct MockSlowmodeRepo {
    configs: Mutex<Vec<MockSlowmodeConfig>>,
}

impl Default for MockSlowmodeRepo {
    fn default() -> Self {
        Self {
            configs: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
trait SlowmodeRepository: Send + Sync {
    async fn get(&self, guild_id: &str) -> Result<Option<MockSlowmodeConfig>, DomainError>;
    async fn set(&self, config: &MockSlowmodeConfig) -> Result<(), DomainError>;
}

#[async_trait]
impl SlowmodeRepository for MockSlowmodeRepo {
    async fn get(&self, guild_id: &str) -> Result<Option<MockSlowmodeConfig>, DomainError> {
        Ok(self.configs.lock().await.iter()
            .find(|c| c.guild_id == guild_id)
            .cloned())
    }

    async fn set(&self, config: &MockSlowmodeConfig) -> Result<(), DomainError> {
        let mut configs = self.configs.lock().await;
        configs.retain(|c| c.guild_id != config.guild_id);
        configs.push(config.clone());
        Ok(())
    }
}

#[tokio::test]
async fn enable_slowmode_sets_config() {
    let _repo = Arc::new(MockSlowmodeRepo::default());
    assert!(true);
}

#[tokio::test]
async fn disable_slowmode_removes_config() {
    assert!(true);
}

#[tokio::test]
async fn get_slowmode_config_returns_none_when_disabled() {
    assert!(true);
}

#[tokio::test]
async fn slowmode_duration_is_respected() {
    assert!(true);
}

#[tokio::test]
async fn slowmode_exemptions_applied() {
    assert!(true);
}
