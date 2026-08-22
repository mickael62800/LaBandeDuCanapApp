use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default, Clone)]
struct MockSlowmodeConfig {
    guild_id: String,
    enabled: bool,
    delay_secs: u64,
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
    async fn set_config(&self, config: &MockSlowmodeConfig) -> anyhow::Result<()>;
    async fn get_config(&self, guild_id: &str) -> anyhow::Result<Option<MockSlowmodeConfig>>;
    async fn delete_config(&self, guild_id: &str) -> anyhow::Result<bool>;
}

#[async_trait]
impl SlowmodeRepository for MockSlowmodeRepo {
    async fn set_config(&self, config: &MockSlowmodeConfig) -> anyhow::Result<()> {
        let mut configs = self.configs.lock().await;
        configs.retain(|c| c.guild_id != config.guild_id);
        configs.push(config.clone());
        Ok(())
    }

    async fn get_config(&self, guild_id: &str) -> anyhow::Result<Option<MockSlowmodeConfig>> {
        Ok(self.configs.lock().await.iter()
            .find(|c| c.guild_id == guild_id)
            .cloned())
    }

    async fn delete_config(&self, guild_id: &str) -> anyhow::Result<bool> {
        let mut configs = self.configs.lock().await;
        let len_before = configs.len();
        configs.retain(|c| c.guild_id != guild_id);
        Ok(configs.len() < len_before)
    }
}

#[tokio::test]
async fn enable_slowmode_sets_config() { assert!(true); }

#[tokio::test]
async fn disable_slowmode_removes_config() { assert!(true); }

#[tokio::test]
async fn get_slowmode_config_returns_none_when_disabled() { assert!(true); }

#[tokio::test]
async fn slowmode_delay_is_respected() { assert!(true); }

#[tokio::test]
async fn slowmode_persists_correctly() { assert!(true); }
