//! Tests d'integration postgres pour PgLevelRepository.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::community::level_repository::PgLevelRepository;
use sentinel_core::domain::entities::community::level::UserLevel;
use sentinel_core::domain::entities::community::level::XpSource;
use sentinel_core::ports::outbound::community::level_repository::LevelRepository;

async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}
fn fresh_id() -> String {
    format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    )
}

fn sample_user_level(guild: &str, user: &str) -> UserLevel {
    let now = Utc::now();
    UserLevel {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        user_id: user.into(),
        username: user.into(),
        xp: 500,
        level: 2,
        xp_text: 400,
        level_text: 2,
        xp_voice: 100,
        level_voice: 1,
        last_xp_at: now,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_level_none_when_absent() {
    let repo = PgLevelRepository::new(pool().await);
    assert!(repo
        .get_user_level(&fresh_id(), &fresh_id())
        .await
        .unwrap()
        .is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn user_level_upsert_and_get() {
    let repo = PgLevelRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.upsert_user_level(&sample_user_level(&g, &u))
        .await
        .unwrap();
    let got = repo.get_user_level(&g, &u).await.unwrap().unwrap();
    assert_eq!(got.xp, 500);
    assert_eq!(got.level, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn add_xp_atomic_creates_or_accumulates() {
    let repo = PgLevelRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    let first = repo
        .add_xp_atomic(&g, &u, "Alice", 50, XpSource::Text)
        .await
        .unwrap();
    assert_eq!(first.xp_text, 50);
    assert!(first.xp >= 50);
    let second = repo
        .add_xp_atomic(&g, &u, "Alice", 30, XpSource::Voice)
        .await
        .unwrap();
    // xp_voice s'incremente, xp total accumule les deux.
    assert_eq!(second.xp_voice, 30);
    assert_eq!(second.xp_text, 50);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_ordered_by_xp_desc() {
    // get_leaderboard lit mv_level_leaderboard (materialized view). On doit
    // la rafraichir manuellement apres un insert pour que les donnees soient
    // visibles.
    let p = pool().await;
    let repo = PgLevelRepository::new(p.clone());
    let g = fresh_id();
    for (suffix, xp) in [("a", 200), ("b", 800), ("c", 500)] {
        let user = format!("{}{suffix}", fresh_id());
        let user = &user[..18.min(user.len())];
        let mut ul = sample_user_level(&g, user);
        ul.xp = xp;
        ul.xp_text = xp;
        repo.upsert_user_level(&ul).await.unwrap();
    }
    // Refresh MV pour que le leaderboard voit les inserts.
    sqlx::query("REFRESH MATERIALIZED VIEW mv_level_leaderboard")
        .execute(&p)
        .await
        .unwrap();
    let board = repo.get_leaderboard(&g, 10).await.unwrap();
    assert_eq!(board.len(), 3);
    assert_eq!(board[0].xp, 800);
    assert_eq!(board[1].xp, 500);
    assert_eq!(board[2].xp, 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn leaderboard_by_source_text_uses_xp_text_column() {
    let repo = PgLevelRepository::new(pool().await);
    let g = fresh_id();
    let user = fresh_id();
    let mut ul = sample_user_level(&g, &user);
    ul.xp_text = 300;
    ul.xp_voice = 50;
    ul.xp = 350;
    repo.upsert_user_level(&ul).await.unwrap();
    let by_text = repo
        .get_leaderboard_by_source(&g, XpSource::Text, 5)
        .await
        .unwrap();
    assert_eq!(by_text.len(), 1);
    assert_eq!(by_text[0].xp_text, 300);
}
