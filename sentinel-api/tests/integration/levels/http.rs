//! Tests d'integration HTTP pour les endpoints levels.

#[path = "../../test_helpers.rs"]
mod test_helpers;

use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use chrono::Utc;
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use sentinel_api::adapters::inbound::http::router;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::community::level::XpSource;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::inbound::community::manage_levels::AddXpCommand;
use sentinel_core::ports::inbound::community::manage_levels::AddXpResult;
use sentinel_core::ports::inbound::community::manage_levels::ManageLevelsUseCase;
use sentinel_core::ports::inbound::community::manage_levels::ResetTarget;
use sentinel_core::ports::inbound::community::manage_levels::SetUserXpCommand;
use test_helpers::build_test_state_levels;

#[derive(Default)]
struct MockLevelsUC {
    users: Mutex<Vec<UserLevel>>,
    last_source: Mutex<Option<XpSource>>,
}

impl MockLevelsUC {
    fn new() -> Self {
        Self::default()
    }
    fn with_user(self, u: UserLevel) -> Self {
        self.users.lock().unwrap().push(u);
        self
    }
}

fn default_user(guild_id: &str, user_id: &str, xp: i64) -> UserLevel {
    let now = Utc::now();
    UserLevel {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        user_id: user_id.into(),
        username: "alice".into(),
        xp,
        level: 1,
        xp_text: xp,
        level_text: 1,
        xp_voice: 0,
        level_voice: 0,
        last_xp_at: now,
        created_at: now,
        updated_at: now,
    }
}

#[async_trait]
impl ManageLevelsUseCase for MockLevelsUC {
    async fn record_text_activity(
        &self,
        _: sentinel_core::ports::inbound::community::manage_levels::RecordTextActivityCommand,
    ) -> Result<
        sentinel_core::ports::inbound::community::manage_levels::RecordActivityResult,
        DomainError,
    > {
        unimplemented!()
    }
    async fn record_voice_activity(
        &self,
        _: sentinel_core::ports::inbound::community::manage_levels::RecordVoiceActivityCommand,
    ) -> Result<
        sentinel_core::ports::inbound::community::manage_levels::RecordActivityResult,
        DomainError,
    > {
        unimplemented!()
    }
    async fn add_xp(&self, cmd: AddXpCommand) -> Result<AddXpResult, DomainError> {
        let user_level = default_user(&cmd.guild_id, &cmd.user_id, cmd.amount);
        Ok(AddXpResult {
            user_level,
            leveled_up: cmd.amount >= 100,
            old_level: 0,
            old_level_global: 0,
            source: cmd.source,
        })
    }
    async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<UserLevel, DomainError> {
        let users = self.users.lock().unwrap();
        Ok(users
            .iter()
            .find(|u| u.guild_id.as_str() == guild_id && u.user_id.as_str() == user_id)
            .cloned()
            .unwrap_or_else(|| default_user(guild_id, user_id, 0)))
    }
    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        let users = self.users.lock().unwrap();
        let mut matching: Vec<UserLevel> = users
            .iter()
            .filter(|u| u.guild_id.as_str() == guild_id)
            .cloned()
            .collect();
        matching.sort_by_key(|u| std::cmp::Reverse(u.xp));
        matching.truncate(limit as usize);
        Ok(matching)
    }
    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError> {
        *self.last_source.lock().unwrap() = Some(source);
        self.get_leaderboard(guild_id, limit).await
    }
    async fn set_user_xp(&self, _: SetUserXpCommand) -> Result<UserLevel, DomainError> {
        unimplemented!()
    }
    async fn reset_user_xp(
        &self,
        _: &str,
        _: &str,
        _: ResetTarget,
    ) -> Result<UserLevel, DomainError> {
        unimplemented!()
    }
}

fn build_app(uc: Arc<MockLevelsUC>) -> axum::Router {
    router::build_for_test(build_test_state_levels(uc))
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

async fn post_json(
    app: axum::Router,
    uri: &str,
    body: serde_json::Value,
) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_returns_user_and_leveled_up_flag() {
    let app = build_app(Arc::new(MockLevelsUC::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "user_id": "u1", "username": "alice",
        "amount": 150, "source": "text"
    });
    let (status, json) = post_json(app, "/api/levels/xp", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["leveled_up"], true);
    assert_eq!(json["source"], "text");
    assert_eq!(json["user"]["xp"], 150);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_defaults_source_to_text() {
    let app = build_app(Arc::new(MockLevelsUC::new()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111", "user_id": "u1", "username": "alice",
        "amount": 50
    });
    let (status, json) = post_json(app, "/api/levels/xp", body).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["source"], "text");
    assert_eq!(json["leveled_up"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_user_level_returns_stored() {
    let uc = MockLevelsUC::new().with_user(default_user("111111111111111111", "u1", 500));
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/levels/111111111111111111/u1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["xp"], 500);
    assert_eq!(json["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_sorted_desc_by_xp() {
    let uc = MockLevelsUC::new()
        .with_user(default_user("111111111111111111", "u1", 100))
        .with_user(default_user("111111111111111111", "u2", 500))
        .with_user(default_user("111111111111111111", "u3", 250));
    let app = build_app(Arc::new(uc));
    let (status, json) = get(app, "/api/levels/111111111111111111/leaderboard").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0]["user_id"], "u2");
    assert_eq!(arr[2]["user_id"], "u1");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_source_voice_calls_by_source_path() {
    let uc = Arc::new(MockLevelsUC::new().with_user(default_user("111111111111111111", "u1", 10)));
    let app = build_app(uc.clone());
    let (status, _) = get(
        app,
        "/api/levels/111111111111111111/leaderboard?source=voice",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(*uc.last_source.lock().unwrap(), Some(XpSource::Voice));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_source_text_calls_by_source_path() {
    let uc = Arc::new(MockLevelsUC::new().with_user(default_user("111111111111111111", "u1", 10)));
    let app = build_app(uc.clone());
    let (status, _) = get(
        app,
        "/api/levels/111111111111111111/leaderboard?source=text",
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(*uc.last_source.lock().unwrap(), Some(XpSource::Text));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_source_none_calls_total_path() {
    let uc = Arc::new(MockLevelsUC::new().with_user(default_user("111111111111111111", "u1", 10)));
    let app = build_app(uc.clone());
    let (status, _) = get(app, "/api/levels/111111111111111111/leaderboard").await;
    assert_eq!(status, StatusCode::OK);
    assert!(uc.last_source.lock().unwrap().is_none());
}
