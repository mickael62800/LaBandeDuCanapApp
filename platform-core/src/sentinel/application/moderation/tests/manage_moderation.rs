//! Tests unitaires du ManageModerationService (use case).
//! Teste la logique metier : log_action, get_history, list_bans, delete_bans_for_user.
//! Utilise des mocks in-memory pour le repo et le cache.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::moderation::manage_moderation_service::ManageModerationService;
use crate::sentinel::domain::entities::moderation::action::applied::*;
use crate::sentinel::domain::entities::moderation::action::strikes::*;
use crate::sentinel::domain::entities::system::rule::*;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_moderation::*;
use crate::sentinel::ports::outbound::moderation::moderation_repository::ModerationRepository;
use crate::sentinel::ports::outbound::moderation::strike_repository::StrikeRepository;
use crate::sentinel::ports::outbound::system::cache::CachePort;
// ══════════════════════════════════════════════════════════
// Mock Repository (in-memory)
// ══════════════════════════════════════════════════════════

struct InMemoryModerationRepo {
    actions: Mutex<Vec<ModerationAction>>,
}

impl InMemoryModerationRepo {
    fn new() -> Self {
        Self {
            actions: Mutex::new(vec![]),
        }
    }
}

#[async_trait]
impl ModerationRepository for InMemoryModerationRepo {
    async fn save(&self, action: &ModerationAction) -> Result<(), DomainError> {
        self.actions.lock().await.push(action.clone());
        Ok(())
    }

    async fn find_by_id(&self, id: uuid::Uuid) -> Result<Option<ModerationAction>, DomainError> {
        let actions = self.actions.lock().await;
        Ok(actions.iter().find(|a| a.id == id).cloned())
    }

    async fn find_by_target(
        &self,
        guild_id: &str,
        target_id: &str,
        _limit: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        let actions = self.actions.lock().await;
        Ok(actions
            .iter()
            .filter(|a| a.guild_id.as_str() == guild_id && a.target_id == target_id)
            .cloned()
            .collect())
    }

    async fn find_bans(
        &self,
        guild_id: Option<&str>,
        _limit: i64,
        _offset: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        let actions = self.actions.lock().await;
        Ok(actions
            .iter()
            .filter(|a| a.action_type.starts_with("ban"))
            .filter(|a| guild_id.is_none_or(|g| a.guild_id.as_str() == g))
            .cloned()
            .collect())
    }

    async fn delete_bans_for_user(
        &self,
        guild_id: &str,
        target_id: &str,
    ) -> Result<(), DomainError> {
        let mut actions = self.actions.lock().await;
        actions.retain(|a| {
            !(a.guild_id.as_str() == guild_id
                && a.target_id == target_id
                && a.action_type.starts_with("ban"))
        });
        Ok(())
    }

    async fn find_all_for_guild(
        &self,
        _: Option<&str>,
        _: i64,
    ) -> Result<Vec<ModerationAction>, DomainError> {
        Ok(vec![])
    }
    async fn delete_action(&self, id: uuid::Uuid) -> Result<bool, DomainError> {
        let mut actions = self.actions.lock().await;
        let len_before = actions.len();
        actions.retain(|a| a.id != id);
        Ok(actions.len() < len_before)
    }
}

// ══════════════════════════════════════════════════════════
// Mock Strike Repository (no-op)
// ══════════════════════════════════════════════════════════

struct NoOpStrikeRepo;

#[async_trait]
impl StrikeRepository for NoOpStrikeRepo {
    async fn save_strike(&self, _: &UserStrike) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_active_strikes(
        &self,
        _: &str,
        _: &str,
        _: i64,
    ) -> Result<Vec<UserStrike>, DomainError> {
        Ok(vec![])
    }
    async fn delete_strikes(&self, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_strike_by_infraction_id(&self, _: uuid::Uuid) -> Result<u64, DomainError> {
        Ok(0)
    }
    async fn get_config(&self, _: &str) -> Result<Option<StrikeConfig>, DomainError> {
        Ok(None)
    }
    async fn save_config(&self, _: &StrikeConfig) -> Result<(), DomainError> {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Mock Cache (no-op)
// ══════════════════════════════════════════════════════════

struct NoOpCache;

#[async_trait]
impl CachePort for NoOpCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        Ok(None)
    }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> {
        Ok(None)
    }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Helper : build service
// ══════════════════════════════════════════════════════════

fn build_service() -> (ManageModerationService, Arc<InMemoryModerationRepo>) {
    let repo = Arc::new(InMemoryModerationRepo::new());
    let cache = Arc::new(NoOpCache);
    let strike_repo = Arc::new(NoOpStrikeRepo);
    let svc = ManageModerationService::new(
        repo.clone() as Arc<dyn ModerationRepository>,
        strike_repo as Arc<dyn StrikeRepository>,
        cache as Arc<dyn CachePort>,
    );
    (svc, repo)
}

fn make_command(
    action_type: &str,
    gravity: Option<&str>,
    duration: Option<u64>,
) -> LogModerationCommand {
    LogModerationCommand {
        guild_id: "guild1".into(),
        channel_id: "chan1".into(),
        moderator_id: "mod1".into(),
        moderator_name: "ModeratorBob".into(),
        target_id: "user1".into(),
        target_name: "Alice".into(),
        action_type: action_type.into(),
        reason: "Test reason".into(),
        gravity: gravity.map(String::from),
        duration,
    }
}

// ══════════════════════════════════════════════════════════
// Tests — log_action
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn log_action_saves_to_repo() {
    let (svc, repo) = build_service();
    let result = svc
        .log_action(make_command("warn", Some("medium"), None))
        .await;
    assert!(result.is_ok());
    let actions = repo.actions.lock().await;
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].action_type, "warn");
    assert_eq!(actions[0].gravity.map(|g| g.as_str()), Some("medium"));
}

#[tokio::test]
async fn log_action_returns_valid_uuid() {
    let (svc, _) = build_service();
    let action = svc
        .log_action(make_command("warn", None, None))
        .await
        .unwrap();
    assert_ne!(action.id, Uuid::nil());
}

#[tokio::test]
async fn log_action_preserves_all_fields() {
    let (svc, _) = build_service();
    let action = svc
        .log_action(LogModerationCommand {
            guild_id: "g1".into(),
            channel_id: "c1".into(),
            moderator_id: "m1".into(),
            moderator_name: "Mod".into(),
            target_id: "t1".into(),
            target_name: "Target".into(),
            action_type: "ban_temp".into(),
            reason: "Raison ici".into(),
            gravity: Some("high".into()),
            duration: Some(7200),
        })
        .await
        .unwrap();
    assert_eq!(action.guild_id.as_str(), "g1");
    assert_eq!(action.channel_id.as_str(), "c1");
    assert_eq!(action.moderator_id, "m1");
    assert_eq!(action.target_id, "t1");
    assert_eq!(action.action_type, "ban_temp");
    assert_eq!(action.reason, "Raison ici");
    assert_eq!(action.gravity.map(|g| g.as_str()), Some("high"));
    assert_eq!(action.duration, Some(7200));
}

// ══════════════════════════════════════════════════════════
// Tests — get_history
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_history_empty_user() {
    let (svc, _) = build_service();
    let history = svc.get_history("guild1", "user1").await.unwrap();
    assert_eq!(history.total_warns, 0);
    assert_eq!(history.total_mutes, 0);
    assert_eq!(history.total_bans, 0);
    assert!(history.actions.is_empty());
}

#[tokio::test]
async fn get_history_counts_correctly() {
    let (svc, _) = build_service();
    svc.log_action(make_command("warn", Some("low"), None))
        .await
        .unwrap();
    svc.log_action(make_command("warn", Some("medium"), None))
        .await
        .unwrap();
    svc.log_action(make_command("mute_temp", None, Some(600)))
        .await
        .unwrap();
    svc.log_action(make_command("ban_permanent", None, None))
        .await
        .unwrap();

    let history = svc.get_history("guild1", "user1").await.unwrap();
    assert_eq!(history.total_warns, 2);
    assert_eq!(history.total_mutes, 1);
    assert_eq!(history.total_bans, 1);
    assert_eq!(history.actions.len(), 4);
}

#[tokio::test]
async fn get_history_counts_mute_types() {
    let (svc, _) = build_service();
    svc.log_action(make_command("mute_temp", None, Some(600)))
        .await
        .unwrap();
    svc.log_action(make_command("mute_permanent", None, None))
        .await
        .unwrap();

    let history = svc.get_history("guild1", "user1").await.unwrap();
    assert_eq!(history.total_mutes, 2);
}

#[tokio::test]
async fn get_history_counts_ban_types() {
    let (svc, _) = build_service();
    svc.log_action(make_command("ban_temp", None, Some(3600)))
        .await
        .unwrap();
    svc.log_action(make_command("ban_permanent", None, None))
        .await
        .unwrap();

    let history = svc.get_history("guild1", "user1").await.unwrap();
    assert_eq!(history.total_bans, 2);
}

#[tokio::test]
async fn get_history_isolates_guilds() {
    let (svc, _) = build_service();
    svc.log_action(make_command("warn", None, None))
        .await
        .unwrap();

    let history = svc.get_history("other_guild", "user1").await.unwrap();
    assert_eq!(history.total_warns, 0);
}

#[tokio::test]
async fn get_history_isolates_users() {
    let (svc, _) = build_service();
    svc.log_action(make_command("warn", None, None))
        .await
        .unwrap();

    let history = svc.get_history("guild1", "other_user").await.unwrap();
    assert_eq!(history.total_warns, 0);
}

// ══════════════════════════════════════════════════════════
// Tests — list_bans
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn list_bans_empty() {
    let (svc, _) = build_service();
    let bans = svc.list_bans(None, 50, 0).await.unwrap();
    assert!(bans.is_empty());
}

#[tokio::test]
async fn list_bans_excludes_non_bans() {
    let (svc, _) = build_service();
    svc.log_action(make_command("warn", Some("low"), None))
        .await
        .unwrap();
    svc.log_action(make_command("mute_temp", None, Some(600)))
        .await
        .unwrap();
    svc.log_action(make_command("ban_permanent", None, None))
        .await
        .unwrap();

    let bans = svc.list_bans(None, 50, 0).await.unwrap();
    assert_eq!(bans.len(), 1);
    assert_eq!(bans[0].action_type, "ban_permanent");
}

#[tokio::test]
async fn list_bans_filters_by_guild() {
    let (svc, _) = build_service();
    svc.log_action(make_command("ban_permanent", None, None))
        .await
        .unwrap();
    svc.log_action(LogModerationCommand {
        guild_id: "guild2".into(),
        channel_id: "c".into(),
        moderator_id: "m".into(),
        moderator_name: "M".into(),
        target_id: "u2".into(),
        target_name: "T2".into(),
        action_type: "ban_permanent".into(),
        reason: "R".into(),
        gravity: None,
        duration: None,
    })
    .await
    .unwrap();

    let bans = svc.list_bans(Some("guild1"), 50, 0).await.unwrap();
    assert_eq!(bans.len(), 1);
    assert_eq!(bans[0].guild_id.as_str(), "guild1");
}

// ══════════════════════════════════════════════════════════
// Tests — delete_bans_for_user
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn delete_bans_removes_ban_entries() {
    let (svc, repo) = build_service();
    svc.log_action(make_command("ban_permanent", None, None))
        .await
        .unwrap();
    svc.log_action(make_command("warn", Some("low"), None))
        .await
        .unwrap();
    assert_eq!(repo.actions.lock().await.len(), 2);

    svc.delete_bans_for_user("guild1", "user1").await.unwrap();

    let remaining = repo.actions.lock().await;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].action_type, "warn");
}

#[tokio::test]
async fn delete_bans_only_for_specific_user() {
    let (svc, _repo) = build_service();
    svc.log_action(make_command("ban_permanent", None, None))
        .await
        .unwrap();
    svc.log_action(LogModerationCommand {
        guild_id: "guild1".into(),
        channel_id: "c".into(),
        moderator_id: "m".into(),
        moderator_name: "M".into(),
        target_id: "user2".into(),
        target_name: "Bob".into(),
        action_type: "ban_permanent".into(),
        reason: "R".into(),
        gravity: None,
        duration: None,
    })
    .await
    .unwrap();

    svc.delete_bans_for_user("guild1", "user1").await.unwrap();

    let bans = svc.list_bans(None, 50, 0).await.unwrap();
    assert_eq!(bans.len(), 1);
    assert_eq!(bans[0].target_id, "user2");
}
