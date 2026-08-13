//! Tests unitaires du ManageStrikesService.
//! Teste la logique metier : add_strike (escalation), get_active_strikes, reset, config.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use chrono::Utc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::sentinel::application::moderation::manage_strikes_service::ManageStrikesService;
use crate::sentinel::domain::entities::moderation::action::strikes::*;
use crate::sentinel::domain::errors::DomainError;
use crate::sentinel::ports::inbound::moderation::manage_strikes::*;
use crate::sentinel::ports::outbound::moderation::strike_repository::StrikeRepository;

// ══════════════════════════════════════════════════════════
// In-memory Strike Repository
// ══════════════════════════════════════════════════════════

struct InMemoryStrikeRepo {
    strikes: Mutex<Vec<UserStrike>>,
    configs: Mutex<HashMap<String, StrikeConfig>>,
}

impl InMemoryStrikeRepo {
    fn new() -> Self {
        Self {
            strikes: Mutex::new(vec![]),
            configs: Mutex::new(HashMap::new()),
        }
    }

    fn with_config(self, config: StrikeConfig) -> Self {
        let mut configs = HashMap::new();
        configs.insert(config.guild_id.to_string(), config);
        Self {
            strikes: self.strikes,
            configs: Mutex::new(configs),
        }
    }
}

#[async_trait]
impl StrikeRepository for InMemoryStrikeRepo {
    async fn save_strike(&self, strike: &UserStrike) -> Result<(), DomainError> {
        self.strikes.lock().await.push(strike.clone());
        Ok(())
    }

    async fn find_active_strikes(
        &self,
        guild_id: &str,
        user_id: &str,
        window_secs: i64,
    ) -> Result<Vec<UserStrike>, DomainError> {
        let now = Utc::now();
        let cutoff = now - Duration::seconds(window_secs);
        let strikes = self.strikes.lock().await;
        Ok(strikes
            .iter()
            .filter(|s| s.guild_id.as_str() == guild_id && s.user_id.as_str() == user_id)
            .filter(|s| s.expires_at.is_none_or(|e| e > now))
            .filter(|s| s.created_at > cutoff)
            .cloned()
            .collect())
    }

    async fn delete_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError> {
        let mut strikes = self.strikes.lock().await;
        strikes.retain(|s| !(s.guild_id.as_str() == guild_id && s.user_id.as_str() == user_id));
        Ok(())
    }

    async fn delete_strike_by_infraction_id(
        &self,
        infraction_id: uuid::Uuid,
    ) -> Result<u64, DomainError> {
        let mut strikes = self.strikes.lock().await;
        let before = strikes.len();
        strikes.retain(|s| s.infraction_id != Some(infraction_id));
        Ok((before - strikes.len()) as u64)
    }

    async fn get_config(&self, guild_id: &str) -> Result<Option<StrikeConfig>, DomainError> {
        let configs = self.configs.lock().await;
        Ok(configs.get(guild_id).cloned())
    }

    async fn save_config(&self, config: &StrikeConfig) -> Result<(), DomainError> {
        let mut configs = self.configs.lock().await;
        configs.insert(config.guild_id.to_string(), config.clone());
        Ok(())
    }
}

// ══════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════

fn build_service() -> ManageStrikesService {
    let repo = Arc::new(InMemoryStrikeRepo::new());
    ManageStrikesService::new(repo as Arc<dyn StrikeRepository>)
}

fn build_service_with_config(config: StrikeConfig) -> ManageStrikesService {
    let repo = Arc::new(InMemoryStrikeRepo::new().with_config(config));
    ManageStrikesService::new(repo as Arc<dyn StrikeRepository>)
}

fn make_config(guild_id: &str, thresholds: Vec<StrikeThreshold>, enabled: bool) -> StrikeConfig {
    StrikeConfig {
        guild_id: guild_id.into(),
        window_secs: 3600,
        thresholds,
        enabled,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn threshold(strikes: u32, action: &str, duration: Option<u64>) -> StrikeThreshold {
    StrikeThreshold {
        strikes,
        action: action.into(),
        duration,
    }
}

fn make_cmd(guild_id: &str, user_id: &str) -> AddStrikeCommand {
    AddStrikeCommand {
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        reason: "Test".into(),
        source: "moderator".into(),
        infraction_id: None,
    }
}

// ══════════════════════════════════════════════════════════
// Tests — add_strike
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn add_strike_saves_to_repo() {
    let svc = build_service();
    let result = svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    assert_eq!(result.strike.guild_id.as_str(), "g1");
    assert_eq!(result.strike.user_id.as_str(), "u1");
    assert_ne!(result.strike.id, Uuid::nil());
    assert_eq!(result.active_count, 1);
}

#[tokio::test]
async fn add_strike_triggers_escalation_at_threshold() {
    let config = make_config("g1", vec![threshold(3, "mute", Some(600))], true);
    let svc = build_service_with_config(config);

    // Add 3 strikes
    svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    let result = svc.add_strike(make_cmd("g1", "u1")).await.unwrap();

    assert_eq!(result.active_count, 3);
    assert_eq!(result.escalation_action.as_deref(), Some("mute"));
    assert_eq!(result.escalation_duration, Some(600));
}

#[tokio::test]
async fn add_strike_picks_highest_matching_threshold() {
    let config = make_config(
        "g1",
        vec![threshold(3, "mute", Some(600)), threshold(5, "ban", None)],
        true,
    );
    let svc = build_service_with_config(config);

    // 4 premiers strikes (mute se declenche au 3e).
    for _ in 0..4 {
        svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    }

    // Le 5e strike franchit le seuil "ban" (plus haut seuil correspondant, pas
    // "mute"). L'escalade n'est emise qu'au franchissement (fire-on-change), donc
    // c'est bien ce 5e strike qui porte l'action "ban", pas les suivants.
    let result = svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    assert_eq!(result.active_count, 5);
    assert_eq!(result.escalation_action.as_deref(), Some("ban"));
    assert_eq!(result.escalation_duration, None);
}

#[tokio::test]
async fn add_strike_no_escalation_below_threshold() {
    let config = make_config("g1", vec![threshold(3, "mute", Some(600))], true);
    let svc = build_service_with_config(config);

    let result = svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    assert_eq!(result.active_count, 1);
    assert!(result.escalation_action.is_none());

    let result = svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    assert_eq!(result.active_count, 2);
    assert!(result.escalation_action.is_none());
}

#[tokio::test]
async fn add_strike_no_escalation_when_disabled() {
    let config = make_config("g1", vec![threshold(1, "ban", None)], false);
    let svc = build_service_with_config(config);

    let result = svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    assert!(result.escalation_action.is_none());
}

// ══════════════════════════════════════════════════════════
// Tests — reset_strikes
// ══════════════════════════════════════════════════════════

/// MOD #5 — `/unwarn` (single + all) s'appuie sur `delete_strike_by_infraction_id`
/// (appele cote API dans `delete_action`) pour retirer UNIQUEMENT le strike lie
/// au warn supprime. Les strikes d'une autre origine (ex. automod, non lies a ce
/// warn) doivent rester intacts — sinon l'escalation serait faussee et le `all`
/// sur-supprimerait.
#[tokio::test]
async fn delete_strike_by_infraction_id_removes_only_linked_strike() {
    let repo = Arc::new(InMemoryStrikeRepo::new());
    let warn_action = Uuid::new_v4();

    // Strike manuel lie au warn.
    repo.save_strike(&UserStrike {
        id: Uuid::new_v4(),
        guild_id: "g1".into(),
        user_id: "u1".into(),
        reason: "warn".into(),
        source: "moderator".into(),
        infraction_id: Some(warn_action),
        expires_at: None,
        created_at: Utc::now(),
    })
    .await
    .unwrap();

    // Strike automod, NON lie a ce warn.
    repo.save_strike(&UserStrike {
        id: Uuid::new_v4(),
        guild_id: "g1".into(),
        user_id: "u1".into(),
        reason: "automod".into(),
        source: "automod".into(),
        infraction_id: None,
        expires_at: None,
        created_at: Utc::now(),
    })
    .await
    .unwrap();

    let removed = repo
        .delete_strike_by_infraction_id(warn_action)
        .await
        .unwrap();
    assert_eq!(removed, 1, "seul le strike lie au warn doit etre retire");

    let remaining = repo.find_active_strikes("g1", "u1", 3600).await.unwrap();
    assert_eq!(remaining.len(), 1, "le strike automod doit subsister");
    assert_eq!(remaining[0].source, "automod");
}

#[tokio::test]
async fn reset_strikes_clears_all() {
    let svc = build_service();
    svc.add_strike(make_cmd("g1", "u1")).await.unwrap();
    svc.add_strike(make_cmd("g1", "u1")).await.unwrap();

    svc.reset_strikes("g1", "u1").await.unwrap();
    let active = svc.get_active_strikes("g1", "u1").await.unwrap();
    assert!(active.is_empty());
}

// ══════════════════════════════════════════════════════════
// Tests — config
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn get_config_returns_defaults() {
    let svc = build_service();
    let config = svc.get_config("g1").await.unwrap();
    assert_eq!(config.guild_id.as_str(), "g1");
    assert_eq!(config.window_secs, 3600);
    assert!(config.thresholds.is_empty());
    assert!(config.enabled);
}

#[tokio::test]
async fn save_config_persists() {
    let svc = build_service();
    let cmd = SaveStrikeConfigCommand {
        guild_id: "g1".into(),
        window_secs: 7200,
        thresholds: vec![threshold(3, "mute", Some(600)), threshold(5, "ban", None)],
        enabled: true,
    };
    svc.save_config(cmd).await.unwrap();

    let config = svc.get_config("g1").await.unwrap();
    assert_eq!(config.window_secs, 7200);
    assert_eq!(config.thresholds.len(), 2);
    assert_eq!(config.thresholds[0].strikes, 3);
    assert_eq!(config.thresholds[1].action, "ban");
}
