//! Tests du service `AssessTargetRiskService` : le SEUIL vient de la config
//! serveur (defaut 7j), la POLITIQUE est appliquee par le domaine pur. Le core
//! ne recoit que les faits Discord.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::system::bot_config::{BotDefinition, BotGuildConfig};
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::assess_target_risk::{
    AssessTargetRiskCommand, AssessTargetRiskUseCase,
};
use crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository;

use super::super::assess_target_risk_service::AssessTargetRiskService;

/// Mock config : renvoie les entrees fournies pour `get_config`.
struct MockConfigRepo {
    entries: Vec<BotGuildConfig>,
}

#[async_trait]
impl BotConfigRepository for MockConfigRepo {
    async fn get_definitions(&self) -> Result<Vec<BotDefinition>, DomainError> {
        Ok(vec![])
    }
    async fn get_config(
        &self,
        _guild_id: &str,
        _bot_name: &str,
    ) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.entries.clone())
    }
    async fn get_all_config(&self, _guild_id: &str) -> Result<Vec<BotGuildConfig>, DomainError> {
        Ok(self.entries.clone())
    }
    async fn set_config(
        &self,
        _guild_id: &str,
        _bot_name: &str,
        _key: &str,
        _value: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(
        &self,
        _guild_id: &str,
        _bot_name: &str,
        _key: &str,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

fn cfg(key: &str, value: &str) -> BotGuildConfig {
    BotGuildConfig {
        id: Uuid::new_v4(),
        guild_id: "123456789012345678".into(),
        bot_name: "moderation-bot".into(),
        config_key: key.into(),
        config_value: value.into(),
        updated_at: chrono::Utc::now(),
    }
}

fn service(entries: Vec<BotGuildConfig>) -> AssessTargetRiskService {
    AssessTargetRiskService::new(Arc::new(MockConfigRepo { entries }))
}

fn cmd(account_age_days: i64, is_bot: bool, has_mod_perms: bool) -> AssessTargetRiskCommand {
    AssessTargetRiskCommand {
        guild_id: "123456789012345678".into(),
        account_age_days,
        is_bot,
        has_mod_perms,
    }
}

#[tokio::test]
async fn default_threshold_recent_account_is_risky() {
    let svc = service(vec![]);
    let d = svc.assess(cmd(3, false, false)).await.unwrap();
    assert!(d.risky);
    assert_eq!(
        d.reason.as_deref(),
        Some("compte Discord cree il y a seulement 3 jour(s)")
    );
}

#[tokio::test]
async fn default_threshold_old_account_is_safe() {
    let svc = service(vec![]);
    let d = svc.assess(cmd(30, false, false)).await.unwrap();
    assert!(!d.risky);
}

#[tokio::test]
async fn bot_target_is_risky() {
    let svc = service(vec![]);
    let d = svc.assess(cmd(365, true, false)).await.unwrap();
    assert_eq!(d.reason.as_deref(), Some("cible est un bot"));
}

#[tokio::test]
async fn mod_member_is_risky() {
    let svc = service(vec![]);
    let d = svc.assess(cmd(365, false, true)).await.unwrap();
    assert_eq!(
        d.reason.as_deref(),
        Some("cible fait partie de l'equipe de moderation")
    );
}

#[tokio::test]
async fn configurable_threshold_overrides_default() {
    // Seuil serveur = 30j : un compte de 10j devient risque.
    let svc = service(vec![cfg("risk_recent_account_days", "30")]);
    let d = svc.assess(cmd(10, false, false)).await.unwrap();
    assert!(d.risky);
}

#[tokio::test]
async fn empty_guild_id_rejected() {
    let svc = service(vec![]);
    let mut bad = cmd(1, false, false);
    bad.guild_id = "  ".into();
    assert!(svc.assess(bad).await.is_err());
}
