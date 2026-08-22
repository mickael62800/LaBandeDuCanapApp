use super::*;
use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
use crate::sentinel::ports::inbound::moderation::manage_rules::CreateRuleCommand;
use crate::sentinel::ports::inbound::moderation::manage_rules::ManageRulesUseCase;
use crate::sentinel::ports::outbound::system::cache::CachePort;
use std::sync::Mutex as StdMutex;

#[derive(Default)]
struct MockRuleRepo {
    saved: StdMutex<Vec<Rule>>,
    by_guild: StdMutex<Vec<Rule>>,
    all: StdMutex<Vec<Rule>>,
    by_id: StdMutex<Option<Rule>>,
    toggle_calls: StdMutex<Vec<(uuid::Uuid, bool)>>,
    delete_calls: StdMutex<Vec<uuid::Uuid>>,
}
#[async_trait]
impl RuleRepository for MockRuleRepo {
    async fn find_by_guild(&self, _: &str) -> Result<Vec<Rule>, DomainError> {
        Ok(self.by_guild.lock().unwrap().clone())
    }
    async fn find_all(&self) -> Result<Vec<Rule>, DomainError> {
        Ok(self.all.lock().unwrap().clone())
    }
    async fn find_by_id(&self, _: uuid::Uuid) -> Result<Option<Rule>, DomainError> {
        Ok(self.by_id.lock().unwrap().clone())
    }
    async fn save(&self, rule: &Rule) -> Result<Rule, DomainError> {
        self.saved.lock().unwrap().push(rule.clone());
        Ok(rule.clone())
    }
    async fn toggle(&self, id: uuid::Uuid, enabled: bool) -> Result<(), DomainError> {
        self.toggle_calls.lock().unwrap().push((id, enabled));
        Ok(())
    }
    async fn delete(&self, id: uuid::Uuid) -> Result<(), DomainError> {
        self.delete_calls.lock().unwrap().push(id);
        Ok(())
    }
    async fn seed_defaults(
        &self,
        _: &[crate::sentinel::domain::entities::system::rule::Rule],
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

#[derive(Default)]
struct SpyCache {
    invalidations: StdMutex<Vec<String>>,
}
#[async_trait]
impl CachePort for SpyCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        Ok(None)
    }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, g: &str) -> Result<(), DomainError> {
        self.invalidations.lock().unwrap().push(g.into());
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

fn make_svc() -> ManageRulesService {
    ManageRulesService::new(Arc::new(MockRuleRepo::default()), Arc::new(NoOpCache))
}

fn valid_cmd() -> CreateRuleCommand {
    CreateRuleCommand {
        guild_id: "g".into(),
        flag_type: FlagType::Spam,
        weight: 3.0,
        threshold_warn: 2.0,
        threshold_delete: 4.0,
        threshold_mute: 6.0,
        threshold_ban: 9.0,
        enabled: true,
    }
}

#[tokio::test]
async fn create_accepts_valid_command() {
    let svc = make_svc();
    let rule = svc.create_or_update_rule(valid_cmd()).await.unwrap();
    assert_eq!(rule.weight, 3.0);
    assert_eq!(rule.threshold_ban, 9.0);
    assert!(rule.enabled);
}

#[tokio::test]
async fn create_rejects_negative_weight() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.weight = -0.1;
    let err = svc.create_or_update_rule(cmd).await.unwrap_err();
    assert!(matches!(err, DomainError::ValidationError(_)));
}

#[tokio::test]
async fn create_accepts_zero_weight() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.weight = 0.0;
    assert!(svc.create_or_update_rule(cmd).await.is_ok());
}

#[tokio::test]
async fn create_rejects_warn_gte_delete() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_warn = 4.0;
    cmd.threshold_delete = 4.0;
    assert!(matches!(
        svc.create_or_update_rule(cmd).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn create_rejects_delete_gte_mute() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_delete = 6.0;
    cmd.threshold_mute = 6.0;
    assert!(matches!(
        svc.create_or_update_rule(cmd).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn create_rejects_mute_gte_ban() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_mute = 9.0;
    cmd.threshold_ban = 9.0;
    assert!(matches!(
        svc.create_or_update_rule(cmd).await,
        Err(DomainError::ValidationError(_))
    ));
}

#[tokio::test]
async fn create_rejects_inverted_thresholds() {
    let svc = make_svc();
    let mut cmd = valid_cmd();
    cmd.threshold_warn = 10.0;
    cmd.threshold_delete = 8.0;
    cmd.threshold_mute = 6.0;
    cmd.threshold_ban = 4.0;
    assert!(matches!(
        svc.create_or_update_rule(cmd).await,
        Err(DomainError::ValidationError(_))
    ));
}

// ── get_rules / get_all_rules ──

fn sample_rule(guild: &str) -> Rule {
    Rule {
        id: uuid::Uuid::new_v4(),
        guild_id: guild.into(),
        flag_type: FlagType::Spam,
        weight: 1.0,
        threshold_warn: 1.0,
        threshold_delete: 2.0,
        threshold_mute: 3.0,
        threshold_ban: 4.0,
        enabled: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn get_rules_returns_guild_rules() {
    let repo = Arc::new(MockRuleRepo::default());
    repo.by_guild.lock().unwrap().push(sample_rule("g1"));
    let svc = ManageRulesService::new(repo, Arc::new(SpyCache::default()));
    let rules = svc.get_rules("g1").await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].guild_id.as_str(), "g1");
}

#[tokio::test]
async fn get_all_rules_returns_all_rules() {
    let repo = Arc::new(MockRuleRepo::default());
    repo.all.lock().unwrap().push(sample_rule("g1"));
    repo.all.lock().unwrap().push(sample_rule("g2"));
    let svc = ManageRulesService::new(repo, Arc::new(SpyCache::default()));
    assert_eq!(svc.get_all_rules().await.unwrap().len(), 2);
}

// ── toggle_rule ──

#[tokio::test]
async fn toggle_rule_delegates_and_invalidates_when_rule_found() {
    let repo = Arc::new(MockRuleRepo::default());
    let cache = Arc::new(SpyCache::default());
    *repo.by_id.lock().unwrap() = Some(sample_rule("g-toggle"));
    let svc = ManageRulesService::new(repo.clone(), cache.clone());
    let id = uuid::Uuid::new_v4();
    let out = svc.toggle_rule(id, false).await.unwrap();
    assert!(!out);
    assert_eq!(repo.toggle_calls.lock().unwrap()[0], (id, false));
    assert_eq!(cache.invalidations.lock().unwrap()[0], "g-toggle");
}

#[tokio::test]
async fn toggle_rule_skips_cache_when_rule_not_found() {
    let repo = Arc::new(MockRuleRepo::default());
    let cache = Arc::new(SpyCache::default());
    // by_id already None by default
    let svc = ManageRulesService::new(repo.clone(), cache.clone());
    let id = uuid::Uuid::new_v4();
    let out = svc.toggle_rule(id, true).await.unwrap();
    assert!(out);
    assert_eq!(repo.toggle_calls.lock().unwrap().len(), 1);
    assert!(cache.invalidations.lock().unwrap().is_empty());
}

// ── delete_rule ──

#[tokio::test]
async fn delete_rule_delegates_and_invalidates() {
    let repo = Arc::new(MockRuleRepo::default());
    let cache = Arc::new(SpyCache::default());
    let svc = ManageRulesService::new(repo.clone(), cache.clone());
    let id = uuid::Uuid::new_v4();
    svc.delete_rule("g-del", id).await.unwrap();
    assert_eq!(repo.delete_calls.lock().unwrap()[0], id);
    assert_eq!(cache.invalidations.lock().unwrap()[0], "g-del");
}

// ── create_or_update invalidates cache ──

#[tokio::test]
async fn create_invalidates_cache_for_guild() {
    let repo = Arc::new(MockRuleRepo::default());
    let cache = Arc::new(SpyCache::default());
    let svc = ManageRulesService::new(repo, cache.clone());
    svc.create_or_update_rule(valid_cmd()).await.unwrap();
    assert_eq!(cache.invalidations.lock().unwrap()[0], "g");
}

#[tokio::test]
async fn delete_rule_with_invalid_guild() {
    let repo = Arc::new(MockRuleRepo::default());
    let cache = Arc::new(SpyCache::default());
    let svc = ManageRulesService::new(repo, cache);
    let id = uuid::Uuid::new_v4();
    let result = svc.delete_rule("", id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn find_by_guild_empty() {
    let repo = Arc::new(MockRuleRepo::default());
    let cache = Arc::new(SpyCache::default());
    let svc = ManageRulesService::new(repo, cache);
    let rules = svc.find_by_guild("empty_guild").await.unwrap();
    assert!(rules.is_empty());
}
