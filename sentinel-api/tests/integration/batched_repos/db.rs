//! Tests d'integration pour BatchedPgLogRepository + BatchedPgAuditLogRepository.
//!
//! Verifie que :
//! - `save()` enqueue et finit par flusher vers Postgres apres l'interval
//! - Les delegates (find_all, delete_by_category, delete_older_than_days)
//!   passent bien au PgLogRepository / PgAuditLogRepository underlying
//! - La config BatchWriter declenche un flush immediat quand max_batch_size
//!   est atteint

use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use sentinel_api::adapters::outbound::batching::audit_log_batcher::BatchedPgAuditLogRepository;
use sentinel_api::adapters::outbound::batching::batch_writer::BatchWriterConfig;
use sentinel_api::adapters::outbound::batching::log_batcher::BatchedPgLogRepository;
use sentinel_core::domain::entities::audit::audit_log::AuditLog;
use ops_core::domain::entities::log_entry::LogEntry;
use sentinel_core::ports::inbound::audit::manage_audit_logs::AuditLogFilters;
use sentinel_core::ports::outbound::audit::audit_log_repository::AuditLogRepository;
use ops_core::ports::outbound::log_repository::LogRepository;
async fn pool() -> PgPool {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://sentinel_test:sentinel_test@localhost:5433/sentinel_test".into()
    });
    PgPool::connect(&url).await.unwrap()
}

fn fresh_tag() -> String {
    // category VARCHAR(20) — on prend seulement 10 chars de l'UUID.
    let uuid = Uuid::new_v4().simple().to_string();
    format!("t{}", &uuid[..10])
}

fn fast_config() -> BatchWriterConfig {
    // Flush tres rapide pour les tests : 100ms.
    BatchWriterConfig {
        flush_interval: Duration::from_millis(100),
        max_batch_size: 50,
        channel_capacity: 1000,
    }
}

fn make_log(bot: &str, category: &str, message: &str) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4(),
        timestamp: chrono::Utc::now(),
        level: "info".into(),
        bot: bot.into(),
        server: "test".into(),
        message: message.into(),
        category: category.into(),
        details: serde_json::json!({}),
    }
}

fn make_audit(guild_id: &str, event_type: &str) -> AuditLog {
    AuditLog {
        id: Uuid::new_v4(),
        guild_id: guild_id.into(),
        event_type: event_type.into(),
        actor_id: None,
        actor_name: None,
        target_id: None,
        target_name: None,
        channel_id: None,
        channel_name: None,
        details: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    }
}

// ══════════════════════════════════════════════════════════════════════
// BatchedPgLogRepository
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn log_save_is_async_enqueue_and_flushes_to_db() {
    let p = pool().await;
    let repo = BatchedPgLogRepository::new(p.clone(), fast_config());
    let category = fresh_tag();

    // save retourne immediatement
    repo.save(&make_log("bot1", &category, "msg1"))
        .await
        .unwrap();
    repo.save(&make_log("bot1", &category, "msg2"))
        .await
        .unwrap();

    // Wait pour que le flush interval se declenche (100ms + marge)
    tokio::time::sleep(Duration::from_millis(300)).await;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM logs WHERE category = $1")
        .bind(&category)
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn log_find_all_delegates_to_inner() {
    let p = pool().await;
    let repo = BatchedPgLogRepository::new(p.clone(), fast_config());
    // find_all n'est pas batched — il lit direct via PgLogRepository.
    // On teste juste qu'il n'erreur pas (retour dependera de ce qu'il y a en DB).
    let _ = repo.find_all(10).await.unwrap();
}

#[tokio::test]
async fn log_delete_by_category_removes_matching_rows() {
    let p = pool().await;
    let repo = BatchedPgLogRepository::new(p.clone(), fast_config());
    let category = fresh_tag();

    // Seed direct en DB
    for i in 0..3 {
        sqlx::query(
            "INSERT INTO logs (id, timestamp, level, bot, server, message, category, details) \
             VALUES ($1, NOW(), 'info', 'b', 's', $2, $3, '{}'::jsonb)",
        )
        .bind(Uuid::new_v4())
        .bind(format!("m{i}"))
        .bind(&category)
        .execute(&p)
        .await
        .unwrap();
    }

    let deleted = repo.delete_by_category(&category).await.unwrap();
    assert_eq!(deleted, 3);

    let remaining: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM logs WHERE category = $1")
        .bind(&category)
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(remaining.0, 0);
}

#[tokio::test]
async fn log_delete_older_than_days_delegates() {
    let p = pool().await;
    let repo = BatchedPgLogRepository::new(p.clone(), fast_config());
    // Ne supprime rien en pratique car on ne seed pas d'anciens logs,
    // mais le delegate doit returner Ok.
    let _ = repo.delete_older_than_days(90).await.unwrap();
}

#[tokio::test]
async fn log_max_batch_size_triggers_immediate_flush() {
    let p = pool().await;
    let cfg = BatchWriterConfig {
        flush_interval: Duration::from_secs(60), // long, pour etre sur que c'est le size qui trigger
        max_batch_size: 5,
        channel_capacity: 100,
    };
    let repo = BatchedPgLogRepository::new(p.clone(), cfg);
    let category = fresh_tag();

    // Enqueue 5 entries → doit flusher immediatement (pas attendre 60s)
    for i in 0..5 {
        repo.save(&make_log("bot", &category, &format!("m{i}")))
            .await
            .unwrap();
    }

    // 500ms est largement suffisant pour l'insert batch + round trip
    tokio::time::sleep(Duration::from_millis(500)).await;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM logs WHERE category = $1")
        .bind(&category)
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(count.0, 5, "max_batch_size doit declencher le flush");
}

// ══════════════════════════════════════════════════════════════════════
// BatchedPgAuditLogRepository
// ══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn audit_save_enqueues_and_flushes() {
    let p = pool().await;
    let repo = BatchedPgAuditLogRepository::new(p.clone(), fast_config());
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );

    repo.save(&make_audit(&guild_id, "evt1")).await.unwrap();
    repo.save(&make_audit(&guild_id, "evt2")).await.unwrap();

    tokio::time::sleep(Duration::from_millis(300)).await;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_logs WHERE guild_id = $1")
        .bind(&guild_id)
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}

#[tokio::test]
async fn audit_find_all_delegates_with_filters() {
    let p = pool().await;
    let repo = BatchedPgAuditLogRepository::new(p.clone(), fast_config());
    let guild_id = format!(
        "{}",
        Uuid::new_v4().as_u128() % 1_000_000_000_000_000_000_u128
    );

    // Seed + attend flush
    for i in 0..3 {
        repo.save(&make_audit(&guild_id, &format!("evt{i}")))
            .await
            .unwrap();
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    let filters = AuditLogFilters {
        event_type: None,
        actor_id: None,
        target_id: None,
        limit: 100,
        offset: 0,
        ..Default::default()
    };
    let logs = repo.find_all(Some(&guild_id), &filters).await.unwrap();
    assert_eq!(logs.len(), 3);
}

#[tokio::test]
async fn audit_delete_older_than_days_delegates() {
    let p = pool().await;
    let repo = BatchedPgAuditLogRepository::new(p, fast_config());
    // Pas d'anciens logs a supprimer → returns 0.
    let deleted = repo
        .delete_older_than_days("nonexistent-guild", 90)
        .await
        .unwrap();
    assert_eq!(deleted, 0);
}
