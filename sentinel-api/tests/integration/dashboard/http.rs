//! Tests d'integration HTTP pour les endpoints dashboard (logs, guilds, bots).

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
use sentinel_core::domain::entities::system::guild::Guild;
use ops_core::domain::entities::log_entry::LogEntry;
use sentinel_core::domain::errors::DomainError;
use sentinel_core::ports::outbound::system::guild_repository::GuildRepository;
use ops_core::ports::outbound::log_repository::LogRepository;
// ══════════════════════════════════════════════════════════
// Mocks
// ══════════════════════════════════════════════════════════

#[derive(Default)]
struct MockLogRepo {
    entries: Mutex<Vec<LogEntry>>,
    deleted_by_category: Mutex<Vec<String>>,
}

impl MockLogRepo {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, e: LogEntry) -> Self {
        self.entries.lock().unwrap().push(e);
        self
    }
}

#[async_trait]
impl LogRepository for MockLogRepo {
    async fn save(&self, entry: &LogEntry) -> Result<(), DomainError> {
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }
    async fn find_all(&self, _limit: i64) -> Result<Vec<LogEntry>, DomainError> {
        Ok(self.entries.lock().unwrap().clone())
    }
    async fn find_filtered(
        &self,
        _category: Option<&str>,
        _level: Option<&str>,
        _guild_id: Option<&str>,
        _limit: i64,
    ) -> Result<Vec<LogEntry>, DomainError> {
        Ok(self.entries.lock().unwrap().clone())
    }
    async fn delete_by_category(&self, category: &str) -> Result<u64, DomainError> {
        self.deleted_by_category
            .lock()
            .unwrap()
            .push(category.into());
        let mut e = self.entries.lock().unwrap();
        let before = e.len();
        e.retain(|x| x.category != category);
        Ok((before - e.len()) as u64)
    }
    async fn delete_older_than_days(&self, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

#[derive(Default)]
struct MockGuildRepo {
    guilds: Mutex<Vec<Guild>>,
}

impl MockGuildRepo {
    fn new() -> Self {
        Self::default()
    }
    fn with(self, g: Guild) -> Self {
        self.guilds.lock().unwrap().push(g);
        self
    }
}

#[async_trait]
impl GuildRepository for MockGuildRepo {
    async fn upsert(&self, guild: &Guild) -> Result<(), DomainError> {
        let mut gs = self.guilds.lock().unwrap();
        gs.retain(|g| g.guild_id != guild.guild_id);
        gs.push(guild.clone());
        Ok(())
    }
    async fn find_all(&self) -> Result<Vec<Guild>, DomainError> {
        Ok(self.guilds.lock().unwrap().clone())
    }
    async fn find_by_id(&self, id: &str) -> Result<Option<Guild>, DomainError> {
        Ok(self
            .guilds
            .lock()
            .unwrap()
            .iter()
            .find(|g| g.guild_id.as_str() == id)
            .cloned())
    }
    async fn delete(&self, id: &str) -> Result<(), DomainError> {
        self.guilds
            .lock()
            .unwrap()
            .retain(|g| g.guild_id.as_str() != id);
        Ok(())
    }
    async fn delete_absent(&self, keep_ids: &[String]) -> Result<u64, DomainError> {
        if keep_ids.is_empty() {
            return Ok(0);
        }
        let mut gs = self.guilds.lock().unwrap();
        let before = gs.len();
        gs.retain(|g| keep_ids.iter().any(|k| k.as_str() == g.guild_id.as_str()));
        Ok((before - gs.len()) as u64)
    }
}

fn sample_log(category: &str, server: &str) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        level: "info".into(),
        bot: "test-bot".into(),
        server: server.into(),
        message: "hello".into(),
        category: category.into(),
        details: serde_json::json!({}),
    }
}

fn sample_guild(id: &str) -> Guild {
    Guild {
        guild_id: id.into(),
        name: format!("Guild {id}"),
        icon: None,
        member_count: 10,
        registered_at: Utc::now(),
        updated_at: Utc::now(),
    }
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

async fn delete(app: axum::Router, uri: &str) -> (StatusCode, serde_json::Value) {
    let req = Request::builder()
        .method("DELETE")
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

// ══════════════════════════════════════════════════════════
// Logs
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_logs_empty() {
    let app = router::build_for_test(test_helpers::build_test_state_logs(Arc::new(
        MockLogRepo::new(),
    )));
    let (status, json) = get(app, "/api/logs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json.as_array().unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_logs_filter_by_guild() {
    let repo = MockLogRepo::new()
        .with(sample_log("bot", "111111111111111111"))
        .with(sample_log("bot", "222222222222222222"));
    let app = router::build_for_test(test_helpers::build_test_state_logs(Arc::new(repo)));
    let (status, json) = get(app, "/api/logs?guild_id=111111111111111111").await;
    assert_eq!(status, StatusCode::OK);
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["server"], "111111111111111111");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_log_201_with_defaults() {
    let repo = Arc::new(MockLogRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_logs(repo.clone()));
    let body = serde_json::json!({"message": "hello"});
    let (status, _) = post_json(app, "/api/logs", body).await;
    assert_eq!(status, StatusCode::CREATED);
    let entries = repo.entries.lock().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].level, "info");
    assert_eq!(entries[0].category, "discord");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_log_infers_category_bot_from_bot_name() {
    let repo = Arc::new(MockLogRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_logs(repo.clone()));
    let body = serde_json::json!({"message": "x", "bot": "moderation-bot"});
    let (status, _) = post_json(app, "/api/logs", body).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(repo.entries.lock().unwrap()[0].category, "bot");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_log_infers_category_worker_from_bot_name() {
    let repo = Arc::new(MockLogRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_logs(repo.clone()));
    let body = serde_json::json!({"message": "x", "bot": "data-worker"});
    let (status, _) = post_json(app, "/api/logs", body).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(repo.entries.lock().unwrap()[0].category, "worker");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_logs_by_category_removes_matching() {
    let repo = Arc::new(
        MockLogRepo::new()
            .with(sample_log("bot", "g"))
            .with(sample_log("bot", "g"))
            .with(sample_log("worker", "g")),
    );
    let app = router::build_for_test(test_helpers::build_test_state_logs(repo.clone()));
    let (status, json) = delete(app, "/api/logs/bot").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json["deleted"], 2);
    assert_eq!(repo.entries.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_logs_category_discord_forbidden() {
    let app = router::build_for_test(test_helpers::build_test_state_logs(Arc::new(
        MockLogRepo::new(),
    )));
    let (status, _) = delete(app, "/api/logs/discord").await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
}

// ══════════════════════════════════════════════════════════
// Guilds
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_guilds_returns_array() {
    // Note : Redis est partage entre les tests du compose, donc on ne peut pas
    // asserter sur une taille precise (le cache `guilds:all` peut venir d'un
    // test precedent). On verifie juste que l'endpoint retourne un array JSON.
    let repo = MockGuildRepo::new().with(sample_guild("111111111111111111"));
    let app = router::build_for_test(test_helpers::build_test_state_guilds(Arc::new(repo)));
    let (status, json) = get(app, "/api/guilds").await;
    assert_eq!(status, StatusCode::OK);
    assert!(json.as_array().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_guild_upserts_and_returns_204() {
    let repo = Arc::new(MockGuildRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_guilds(repo.clone()));
    let body = serde_json::json!({
        "guild_id": "111111111111111111",
        "name": "My Guild",
        "member_count": 42
    });
    let (status, _) = post_json(app, "/api/guilds/register", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let g = repo.guilds.lock().unwrap();
    assert_eq!(g.len(), 1);
    assert_eq!(g[0].name, "My Guild");
    assert_eq!(g[0].member_count, 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_guild_member_count_defaults_to_zero() {
    let repo = Arc::new(MockGuildRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_guilds(repo.clone()));
    let body = serde_json::json!({"guild_id": "111111111111111111", "name": "X"});
    let (status, _) = post_json(app, "/api/guilds/register", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(repo.guilds.lock().unwrap()[0].member_count, 0);
}

// ══════════════════════════════════════════════════════════
// Bot heartbeat
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn bot_heartbeat_returns_204() {
    // Redis sur compose test : SET + SADD s'executent reellement.
    let app = router::build_for_test(test_helpers::build_test_state_logs(Arc::new(
        MockLogRepo::new(),
    )));
    let body = serde_json::json!({"name": "automod-bot"});
    let (status, _) = post_json(app, "/api/bots/heartbeat", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

// ══════════════════════════════════════════════════════════
// Chemins Redis cache-hit et sqlx direct (utilisent compose test)
// ══════════════════════════════════════════════════════════

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_guilds_second_call_hits_cache() {
    // Premier appel populate le cache Redis 'guilds:all'. Second appel hit
    // le cache et deserialise le JSON -> couvre la branche cache-hit.
    // Note : le cache est partage entre tests → on verifie juste que le 2e
    // appel reussit (le cache est populate par l'un quelconque des tests).
    let repo = Arc::new(MockGuildRepo::new().with(sample_guild("333333333333333333")));
    let app1 = router::build_for_test(test_helpers::build_test_state_guilds(repo.clone()));
    let (s, _) = get(app1, "/api/guilds").await;
    assert_eq!(s, StatusCode::OK);
    let app2 = router::build_for_test(test_helpers::build_test_state_guilds(repo.clone()));
    let (s, json) = get(app2, "/api/guilds").await;
    assert_eq!(s, StatusCode::OK);
    assert!(json.as_array().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn register_guild_with_owner_id_triggers_rbac_insert() {
    // Couvre la branche `if let Some(owner) = owner_id` qui fait un INSERT
    // direct sqlx dans api_user_guilds (ON CONFLICT DO NOTHING).
    let repo = Arc::new(MockGuildRepo::new());
    let app = router::build_for_test(test_helpers::build_test_state_guilds(repo.clone()));
    let guild_id = format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let owner_id = format!(
        "{}",
        uuid::Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );
    let body = serde_json::json!({
        "guild_id": guild_id, "name": "Owned", "owner_id": owner_id
    });
    let (status, _) = post_json(app, "/api/guilds/register", body).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(repo.guilds.lock().unwrap().len(), 1);
}
