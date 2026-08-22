use super::*;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::moderation::manage_rules_service::ManageRulesService;
use crate::sentinel::domain::entities::system::rule::Rule;
use crate::sentinel::domain::enums::moderation::flag_type::FlagType;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_rules::{CreateRuleCommand, ManageRulesUseCase};
use crate::sentinel::ports::outbound::system::cache::CachePort;
use crate::sentinel::ports::outbound::system::rule_repository::RuleRepository;
use async_trait::async_trait;
use chrono::Utc;

#[derive(Default)]
struct MockRuleRepo {
    rules: Mutex<Vec<Rule>>,
}

#[async_trait]
impl RuleRepository for MockRuleRepo {
    async fn find_by_guild(&self, guild_id: &str) -> Result<Vec<Rule>, DomainError> {
        Ok(self.rules.lock().unwrap().iter()
            .filter(|r| r.guild_id == guild_id)
            .cloned().collect())
    }
    async fn find_all(&self) -> Result<Vec<Rule>, DomainError> {
        Ok(self.rules.lock().unwrap().clone())
    }
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Rule>, DomainError> {
        Ok(self.rules.lock().unwrap().iter().find(|r| r.id == id).cloned())
    }
    async fn save(&self, rule: &Rule) -> Result<Rule, DomainError> {
        self.rules.lock().unwrap().push(rule.clone());
        Ok(rule.clone())
    }
    async fn toggle(&self, _id: Uuid, _enabled: bool) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete(&self, id: Uuid) -> Result<(), DomainError> {
        self.rules.lock().unwrap().retain(|r| r.id != id);
        Ok(())
    }
    async fn seed_defaults(&self, _rules: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
}

struct NoOpCache;
#[async_trait]
impl CachePort for NoOpCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> { Ok(None) }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn get_json(&self, _: &str) -> Result<Option<String>, DomainError> { Ok(None) }
    async fn set_json(&self, _: &str, _: &str, _: u64) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> { Ok(()) }
}

fn valid_cmd() -> CreateRuleCommand {
    CreateRuleCommand {
        guild_id: "g1".into(),
        flag_type: FlagType::Spam,
        weight: 1.0,
        threshold_warn: 1.0,
        threshold_delete: 2.0,
        threshold_mute: 3.0,
        threshold_ban: 4.0,
        enabled: true,
    }
}

#[tokio::test]
async fn create_rule_valid() {
    let svc = ManageRulesService::new(Arc::new(MockRuleRepo::default()), Arc::new(NoOpCache));
    let result = svc.create_or_update_rule(valid_cmd()).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn create_rule_negative_weight() {
    let svc = ManageRulesService::new(Arc::new(MockRuleRepo::default()), Arc::new(NoOpCache));
    let mut cmd = valid_cmd();
    cmd.weight = -1.0;
    let result = svc.create_or_update_rule(cmd).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_rules_by_guild() {
    let svc = ManageRulesService::new(Arc::new(MockRuleRepo::default()), Arc::new(NoOpCache));
    let result = svc.get_rules("g1").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn delete_rule() {
    let svc = ManageRulesService::new(Arc::new(MockRuleRepo::default()), Arc::new(NoOpCache));
    let id = Uuid::new_v4();
    let result = svc.delete_rule("g1", id).await;
    assert!(result.is_ok());
}
