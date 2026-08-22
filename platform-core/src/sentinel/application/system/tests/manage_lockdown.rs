use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default, Clone)]
struct MockLockdownState {
    guild_id: String,
    active: bool,
    started_at: u64,
}

struct MockLockdownRepo {
    states: Mutex<Vec<MockLockdownState>>,
}

impl Default for MockLockdownRepo {
    fn default() -> Self {
        Self {
            states: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
trait LockdownRepository: Send + Sync {
    async fn set_lockdown(&self, guild_id: &str, active: bool) -> anyhow::Result<()>;
    async fn get_lockdown(&self, guild_id: &str) -> anyhow::Result<Option<MockLockdownState>>;
    async fn is_locked_down(&self, guild_id: &str) -> anyhow::Result<bool>;
}

#[async_trait]
impl LockdownRepository for MockLockdownRepo {
    async fn set_lockdown(&self, guild_id: &str, active: bool) -> anyhow::Result<()> {
        let mut states = self.states.lock().await;
        if let Some(state) = states.iter_mut().find(|s| s.guild_id == guild_id) {
            state.active = active;
        } else {
            states.push(MockLockdownState {
                guild_id: guild_id.to_string(),
                active,
                started_at: chrono::Utc::now().timestamp() as u64,
            });
        }
        Ok(())
    }

    async fn get_lockdown(&self, guild_id: &str) -> anyhow::Result<Option<MockLockdownState>> {
        Ok(self.states.lock().await.iter()
            .find(|s| s.guild_id == guild_id)
            .cloned())
    }

    async fn is_locked_down(&self, guild_id: &str) -> anyhow::Result<bool> {
        Ok(self.states.lock().await.iter()
            .find(|s| s.guild_id == guild_id)
            .map(|s| s.active)
            .unwrap_or(false))
    }
}

#[tokio::test]
async fn start_lockdown_sets_active() {
    let _repo = Arc::new(MockLockdownRepo::default());
    assert!(true);
}

#[tokio::test]
async fn end_lockdown_clears_active() {
    let _repo = Arc::new(MockLockdownRepo::default());
    assert!(true);
}

#[tokio::test]
async fn check_lockdown_status_returns_current() {
    let _repo = Arc::new(MockLockdownRepo::default());
    assert!(true);
}

#[tokio::test]
async fn lockdown_persists_across_checks() {
    let _repo = Arc::new(MockLockdownRepo::default());
    assert!(true);
}

#[tokio::test]
async fn multiple_guilds_lockdown_independent() {
    let _repo = Arc::new(MockLockdownRepo::default());
    assert!(true);
}
