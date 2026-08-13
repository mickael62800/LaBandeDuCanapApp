use super::*;
use crate::sentinel::domain::entities::community::level::UserLevel;
use crate::sentinel::domain::entities::community::level::XpSource;
use crate::sentinel::ports::inbound::community::manage_levels::AddXpCommand;
use crate::sentinel::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use chrono::Utc as ChronoUtc;
use std::sync::Mutex as StdMutex;

struct MockRepo {
    user_level: StdMutex<Option<UserLevel>>,
}
impl MockRepo {
    fn new() -> Self {
        Self {
            user_level: StdMutex::new(None),
        }
    }
}

#[async_trait]
impl LevelRepository for MockRepo {
    async fn add_xp_atomic(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        amount: i64,
        source: XpSource,
    ) -> Result<UserLevel, DomainError> {
        let now = ChronoUtc::now();
        let (xp_text, xp_voice) = match source {
            XpSource::Text => (amount, 0),
            XpSource::Voice => (0, amount),
        };
        Ok(UserLevel {
            id: uuid::Uuid::new_v4(),
            guild_id: guild_id.into(),
            user_id: user_id.into(),
            username: username.into(),
            xp: amount,
            level: 0,
            xp_text,
            level_text: 0,
            xp_voice,
            level_voice: 0,
            last_xp_at: now,
            created_at: now,
            updated_at: now,
        })
    }
    async fn upsert_user_level(&self, user_level: &UserLevel) -> Result<(), DomainError> {
        *self.user_level.lock().unwrap() = Some(user_level.clone());
        Ok(())
    }
    async fn get_user_level(&self, _g: &str, _u: &str) -> Result<Option<UserLevel>, DomainError> {
        Ok(self.user_level.lock().unwrap().clone())
    }
    async fn get_leaderboard(&self, _: &str, _: i64) -> Result<Vec<UserLevel>, DomainError> {
        Ok(vec![])
    }
    async fn get_leaderboard_by_source(
        &self,
        _: &str,
        _: XpSource,
        _: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        Ok(vec![])
    }
    async fn refresh_leaderboard_view(&self) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_streak(
        &self,
        _g: &str,
        _u: &str,
    ) -> Result<
        Option<crate::sentinel::domain::entities::community::progression_calc::StreakState>,
        DomainError,
    > {
        Ok(None)
    }
    async fn update_streak(
        &self,
        _g: &str,
        _u: &str,
        _s: crate::sentinel::domain::entities::community::progression_calc::StreakState,
    ) -> Result<(), DomainError> {
        Ok(())
    }
}

struct MockBotConfig;
#[async_trait]
impl crate::sentinel::ports::outbound::system::bot_config_repository::BotConfigRepository
    for MockBotConfig
{
    async fn get_definitions(
        &self,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::bot_config::BotDefinition>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn get_config(
        &self,
        _g: &str,
        _b: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::bot_config::BotGuildConfig>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn get_all_config(
        &self,
        _g: &str,
    ) -> Result<
        Vec<crate::sentinel::domain::entities::system::bot_config::BotGuildConfig>,
        DomainError,
    > {
        Ok(vec![])
    }
    async fn set_config(&self, _: &str, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn delete_config(&self, _: &str, _: &str, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

fn make_svc() -> ManageLevelsService {
    ManageLevelsService::new(
        std::sync::Arc::new(MockRepo::new()),
        std::sync::Arc::new(MockBotConfig),
    )
}

#[tokio::test]
async fn add_xp_rejects_non_positive_amount() {
    let svc = make_svc();
    assert!(svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: 0,
            source: XpSource::Text,
        })
        .await
        .is_err());
    assert!(svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: -5,
            source: XpSource::Text,
        })
        .await
        .is_err());
}

#[tokio::test]
async fn add_xp_rejects_above_cap() {
    let svc = make_svc();
    assert!(svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: 10001,
            source: XpSource::Text,
        })
        .await
        .is_err());
}

#[tokio::test]
async fn add_xp_accepts_within_range() {
    let svc = make_svc();
    let result = svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: 100,
            source: XpSource::Text,
        })
        .await
        .unwrap();
    assert_eq!(result.user_level.xp, 100);
    assert_eq!(result.source, XpSource::Text);
}

#[tokio::test]
async fn add_xp_boundary_10000_accepted() {
    let svc = make_svc();
    assert!(svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: 10000,
            source: XpSource::Voice,
        })
        .await
        .is_ok());
}

#[tokio::test]
async fn get_user_level_not_found_when_repo_empty() {
    let svc = make_svc();
    let err = svc.get_user_level("g", "ghost").await.unwrap_err();
    assert!(matches!(err, DomainError::NotFound(_)));
}

#[tokio::test]
async fn get_user_level_found_after_add_xp() {
    let svc = make_svc();
    svc.add_xp(AddXpCommand {
        guild_id: "g".into(),
        user_id: "u1".into(),
        username: "u1".into(),
        amount: 50,
        source: XpSource::Text,
    })
    .await
    .unwrap();
    let ul = svc.get_user_level("g", "u1").await.unwrap();
    assert_eq!(ul.xp, 50);
    assert_eq!(ul.xp_text, 50);
}

#[tokio::test]
async fn get_leaderboard_passes_through_repo() {
    let svc = make_svc();
    let res = svc.get_leaderboard("g", 10).await.unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn get_leaderboard_by_source_voice() {
    let svc = make_svc();
    let res = svc
        .get_leaderboard_by_source("g", XpSource::Voice, 5)
        .await
        .unwrap();
    assert!(res.is_empty());
}

#[tokio::test]
async fn add_xp_text_source_updates_only_xp_text() {
    let svc = make_svc();
    let res = svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: 200,
            source: XpSource::Text,
        })
        .await
        .unwrap();
    assert_eq!(res.user_level.xp_text, 200);
    assert_eq!(res.user_level.xp_voice, 0);
    assert_eq!(res.source, XpSource::Text);
}

#[tokio::test]
async fn add_xp_voice_source_updates_only_xp_voice() {
    let svc = make_svc();
    let res = svc
        .add_xp(AddXpCommand {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "u".into(),
            amount: 300,
            source: XpSource::Voice,
        })
        .await
        .unwrap();
    assert_eq!(res.user_level.xp_text, 0);
    assert_eq!(res.user_level.xp_voice, 300);
    assert_eq!(res.source, XpSource::Voice);
}
