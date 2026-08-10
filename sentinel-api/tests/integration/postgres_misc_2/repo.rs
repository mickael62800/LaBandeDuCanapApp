//! Tests d'integration postgres pour 4 repos simples :
//! discord_role, log, user_activity, rule. Pure plomberie CRUD.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use sentinel_api::adapters::outbound::postgres::audit::user_activity_repository::PgUserActivityRepository;
use sentinel_api::adapters::outbound::postgres::community::discord_role_repository::PgDiscordRoleRepository;
use sentinel_api::adapters::outbound::postgres::moderation::rule_repository::PgRuleRepository;
use sentinel_api::adapters::outbound::postgres::system::log_repository::PgLogRepository;
use sentinel_core::domain::entities::audit::user_activity::UserActivity;
use sentinel_core::domain::entities::system::discord_role::DiscordRole;
use ops_core::domain::entities::log_entry::LogEntry;
use sentinel_core::domain::entities::system::rule::Rule;
use sentinel_core::domain::enums::moderation::flag_type::FlagType;
use sentinel_core::ports::outbound::audit::user_activity_repository::UserActivityRepository;
use sentinel_core::ports::outbound::community::discord_role_repository::DiscordRoleRepository;
use sentinel_core::ports::outbound::moderation::rule_repository::RuleRepository;
use ops_core::ports::outbound::log_repository::LogRepository;
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

// ══════════════════════════════════════════════════════════
// DiscordRole
// ══════════════════════════════════════════════════════════

fn role(guild: &str, role_id: &str, name: &str, pos: i32) -> DiscordRole {
    DiscordRole {
        id: role_id.into(),
        guild_id: guild.into(),
        name: name.into(),
        color: 0x7289DA,
        position: pos,
        permissions: 0,
        mentionable: true,
        managed: false,
        icon: None,
        member_count: 0,
        synced_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_sync_replaces_all() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    let g = fresh_id();
    repo.sync_roles(
        &g,
        vec![
            role(&g, &fresh_id(), "Admin", 10),
            role(&g, &fresh_id(), "Member", 1),
        ],
    )
    .await
    .unwrap();
    assert_eq!(repo.find_by_guild(&g).await.unwrap().len(), 2);
    // Resync remplace totalement.
    let new_id = fresh_id();
    repo.sync_roles(&g, vec![role(&g, &new_id, "Only", 5)])
        .await
        .unwrap();
    let roles = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].id, new_id);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_find_by_guild_ordered_desc() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    let g = fresh_id();
    repo.sync_roles(
        &g,
        vec![
            role(&g, &fresh_id(), "Low", 1),
            role(&g, &fresh_id(), "High", 100),
            role(&g, &fresh_id(), "Mid", 50),
        ],
    )
    .await
    .unwrap();
    let roles = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(roles[0].position, 100);
    assert_eq!(roles[2].position, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_find_by_id() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    let g = fresh_id();
    let rid = fresh_id();
    repo.sync_roles(&g, vec![role(&g, &rid, "X", 0)])
        .await
        .unwrap();
    assert!(repo.find_by_id(&g, &rid).await.unwrap().is_some());
    assert!(repo.find_by_id(&g, "nonexistent").await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_sync_empty_list_clears_guild() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    let g = fresh_id();
    repo.sync_roles(&g, vec![role(&g, &fresh_id(), "A", 1)])
        .await
        .unwrap();
    repo.sync_roles(&g, vec![]).await.unwrap();
    assert!(repo.find_by_guild(&g).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_find_by_guild_empty_returns_empty_vec() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    assert!(repo.find_by_guild(&fresh_id()).await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_preserves_all_fields() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    let g = fresh_id();
    let rid = fresh_id();
    let r = DiscordRole {
        id: rid.clone(),
        guild_id: g.clone().into(),
        name: "Special".into(),
        color: 0xFF0000,
        position: 42,
        permissions: 0x8, // ADMINISTRATOR
        mentionable: false,
        managed: true,
        icon: Some("icon_hash".into()),
        member_count: 99,
        synced_at: Utc::now(),
    };
    repo.sync_roles(&g, vec![r]).await.unwrap();
    let got = repo.find_by_id(&g, &rid).await.unwrap().unwrap();
    assert_eq!(got.name, "Special");
    assert_eq!(got.color, 0xFF0000);
    assert_eq!(got.position, 42);
    assert_eq!(got.permissions, 0x8);
    assert!(!got.mentionable);
    assert!(got.managed);
    assert_eq!(got.icon.as_deref(), Some("icon_hash"));
    assert_eq!(got.member_count, 99);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discord_role_sync_isolates_per_guild() {
    let repo = PgDiscordRoleRepository::new(pool().await);
    let g1 = fresh_id();
    let g2 = fresh_id();
    repo.sync_roles(&g1, vec![role(&g1, &fresh_id(), "G1-Role", 1)])
        .await
        .unwrap();
    repo.sync_roles(&g2, vec![role(&g2, &fresh_id(), "G2-Role", 1)])
        .await
        .unwrap();
    assert_eq!(repo.find_by_guild(&g1).await.unwrap().len(), 1);
    assert_eq!(repo.find_by_guild(&g2).await.unwrap().len(), 1);
    // Resync g1 doesn't affect g2
    repo.sync_roles(&g1, vec![]).await.unwrap();
    assert!(repo.find_by_guild(&g1).await.unwrap().is_empty());
    assert_eq!(repo.find_by_guild(&g2).await.unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════
// LogEntry
// ══════════════════════════════════════════════════════════

fn log_entry(category: &str, message: &str) -> LogEntry {
    LogEntry {
        id: Uuid::new_v4(),
        timestamp: Utc::now(),
        level: "info".into(),
        bot: "test-bot".into(),
        server: "test-server".into(),
        message: message.into(),
        category: category.into(),
        details: serde_json::json!({}),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_save_and_find_all_desc() {
    let repo = PgLogRepository::new(pool().await);
    // Ecrire un log unique qu'on peut retrouver.
    let marker = format!("marker-{}", Uuid::new_v4());
    repo.save(&log_entry("test-cat", &marker)).await.unwrap();
    let all = repo.find_all(100).await.unwrap();
    assert!(all.iter().any(|l| l.message == marker));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_delete_by_category_returns_count() {
    let repo = PgLogRepository::new(pool().await);
    // category est VARCHAR(20) — utilise un prefix + 8 hex chars.
    let cat = format!("del-{:08x}", Uuid::new_v4().as_u128() as u32);
    repo.save(&log_entry(&cat, "m1")).await.unwrap();
    repo.save(&log_entry(&cat, "m2")).await.unwrap();
    let n = repo.delete_by_category(&cat).await.unwrap();
    assert_eq!(n, 2);
    assert_eq!(repo.delete_by_category(&cat).await.unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn log_delete_older_than_days() {
    let p = pool().await;
    let repo = PgLogRepository::new(p.clone());
    let cat = format!("old-{:08x}", Uuid::new_v4().as_u128() as u32);
    // Insert direct avec timestamp ancien.
    sqlx::query(
        "INSERT INTO logs (id, timestamp, level, bot, server, message, category, details) \
         VALUES ($1, NOW() - INTERVAL '10 days', 'info', 'b', 's', 'old', $2, '{}')",
    )
    .bind(Uuid::new_v4())
    .bind(&cat)
    .execute(&p)
    .await
    .unwrap();
    // Un autre recent.
    repo.save(&log_entry(&cat, "recent")).await.unwrap();

    let n = repo.delete_older_than_days(7).await.unwrap();
    assert!(n >= 1);
}

// ══════════════════════════════════════════════════════════
// UserActivity
// ══════════════════════════════════════════════════════════

fn activity(guild: &str, user: &str, event: &str) -> UserActivity {
    UserActivity {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        user_id: user.into(),
        event_type: event.into(),
        channel_id: Some("ch1".into()),
        channel_name: Some("general".into()),
        content: Some("test".into()),
        metadata: serde_json::json!({}),
        created_at: Utc::now(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_create_and_list_scoped() {
    let repo = PgUserActivityRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.create(&activity(&g, &u, "message")).await.unwrap();
    repo.create(&activity(&g, &u, "message")).await.unwrap();
    repo.create(&activity(&g, &u, "voice_join")).await.unwrap();
    let all = repo.list(&g, &u, None, 50, 0).await.unwrap();
    assert_eq!(all.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_filter_by_event_type() {
    let repo = PgUserActivityRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    repo.create(&activity(&g, &u, "message")).await.unwrap();
    repo.create(&activity(&g, &u, "voice_join")).await.unwrap();
    let msgs = repo.list(&g, &u, Some("message"), 50, 0).await.unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].event_type, "message");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn activity_list_pagination() {
    let repo = PgUserActivityRepository::new(pool().await);
    let g = fresh_id();
    let u = fresh_id();
    for _ in 0..5 {
        repo.create(&activity(&g, &u, "message")).await.unwrap();
    }
    let page1 = repo.list(&g, &u, None, 2, 0).await.unwrap();
    let page2 = repo.list(&g, &u, None, 2, 2).await.unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page2.len(), 2);
}

// ══════════════════════════════════════════════════════════
// Rule
// ══════════════════════════════════════════════════════════

fn sample_rule(guild: &str, flag: FlagType) -> Rule {
    let now = Utc::now();
    Rule {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        flag_type: flag,
        weight: 1.0,
        threshold_warn: 0.3,
        threshold_delete: 0.5,
        threshold_mute: 0.7,
        threshold_ban: 0.9,
        enabled: true,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_save_and_find_by_guild() {
    let repo = PgRuleRepository::new(pool().await);
    let g = fresh_id();
    let r = sample_rule(&g, FlagType::Spam);
    repo.save(&r).await.unwrap();
    let by_guild = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(by_guild.len(), 1);
    assert_eq!(by_guild[0].flag_type, FlagType::Spam);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_save_upsert_on_guild_flag() {
    let repo = PgRuleRepository::new(pool().await);
    let g = fresh_id();
    let mut r = sample_rule(&g, FlagType::Spam);
    repo.save(&r).await.unwrap();
    r.weight = 2.5;
    repo.save(&r).await.unwrap();
    let all = repo.find_by_guild(&g).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].weight, 2.5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_toggle_updates_enabled() {
    let repo = PgRuleRepository::new(pool().await);
    let g = fresh_id();
    let r = sample_rule(&g, FlagType::Insult);
    let saved = repo.save(&r).await.unwrap();
    repo.toggle(saved.id, false).await.unwrap();
    let by_id = repo.find_by_id(saved.id).await.unwrap().unwrap();
    assert!(!by_id.enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_toggle_unknown_returns_not_found() {
    let repo = PgRuleRepository::new(pool().await);
    let err = repo.toggle(Uuid::new_v4(), true).await.unwrap_err();
    assert!(matches!(
        err,
        sentinel_core::domain::errors::DomainError::NotFound(_)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_delete_returns_not_found_on_unknown() {
    let repo = PgRuleRepository::new(pool().await);
    let err = repo.delete(Uuid::new_v4()).await.unwrap_err();
    assert!(matches!(
        err,
        sentinel_core::domain::errors::DomainError::NotFound(_)
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_delete_existing() {
    let repo = PgRuleRepository::new(pool().await);
    let g = fresh_id();
    let saved = repo.save(&sample_rule(&g, FlagType::Link)).await.unwrap();
    repo.delete(saved.id).await.unwrap();
    assert!(repo.find_by_id(saved.id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rule_find_all_returns_rows() {
    let repo = PgRuleRepository::new(pool().await);
    let g = fresh_id();
    repo.save(&sample_rule(&g, FlagType::Spam)).await.unwrap();
    // find_all scope all — on verifie juste que notre insert est visible.
    let all = repo.find_all().await.unwrap();
    assert!(all.iter().any(|r| r.guild_id.as_str() == g));
}
