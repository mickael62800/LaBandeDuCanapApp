use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;
use std::sync::Mutex;
use uuid::Uuid;

use crate::application::audit::manage_stats_service::ManageStatsService;
use crate::domain::entities::audit::user_stats::UserStats;
use crate::domain::entities::audit::user_stats::VoiceSessionStats;
use crate::domain::entities::moderation::detection_flags::DetectionFlags;
use crate::domain::entities::moderation::infraction::Infraction;
use crate::domain::entities::system::rule::Rule;
use crate::domain::enums::moderation::action::Action;
use crate::domain::errors::DomainError;
use crate::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use crate::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use crate::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use crate::ports::inbound::moderation::manage_infractions::InfractionFilters;
use crate::ports::outbound::audit::stats_repository::StatsRepository;
use crate::ports::outbound::moderation::infraction_repository::InfractionRepository;
use crate::ports::outbound::system::cache::CachePort;

// ── MockStatsRepo ──

#[derive(Default)]
struct MockStatsRepo {
    inc_messages: Mutex<Vec<(String, String, String, u64)>>,
    add_voice: Mutex<Vec<(String, String, String, u64)>>,
    voice_sessions: Mutex<Vec<(String, String, String, String, String, u64)>>,
    find_by_user_returns: Mutex<Option<UserStats>>,
    find_by_guild_returns: Mutex<Vec<UserStats>>,
    voice_stats_returns: Mutex<Vec<VoiceSessionStats>>,
    unique_voice_users: Mutex<i64>,
}

fn sample_stats(guild: &str, user: &str, msgs: u64, voice: u64) -> UserStats {
    UserStats {
        id: Uuid::new_v4(),
        guild_id: guild.into(),
        user_id: user.into(),
        username: user.into(),
        message_count: msgs,
        voice_seconds: voice,
        updated_at: Utc::now(),
    }
}

#[async_trait]
impl StatsRepository for MockStatsRepo {
    async fn upsert(&self, _: &UserStats) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_by_user(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> {
        Ok(self.find_by_user_returns.lock().unwrap().clone())
    }
    async fn find_by_guild(&self, _: &str, _: u32) -> Result<Vec<UserStats>, DomainError> {
        Ok(self.find_by_guild_returns.lock().unwrap().clone())
    }
    async fn increment_messages(
        &self,
        g: &str,
        u: &str,
        n: &str,
        c: u64,
    ) -> Result<(), DomainError> {
        self.inc_messages
            .lock()
            .unwrap()
            .push((g.into(), u.into(), n.into(), c));
        Ok(())
    }
    async fn add_voice_seconds(
        &self,
        g: &str,
        u: &str,
        n: &str,
        s: u64,
    ) -> Result<(), DomainError> {
        self.add_voice
            .lock()
            .unwrap()
            .push((g.into(), u.into(), n.into(), s));
        Ok(())
    }
    async fn count_distinct_guilds(&self) -> Result<u64, DomainError> {
        Ok(3)
    }
    async fn count_distinct_users(&self) -> Result<u64, DomainError> {
        Ok(25)
    }
    async fn save_voice_session(
        &self,
        g: &str,
        u: &str,
        n: &str,
        c: &str,
        cn: &str,
        d: u64,
    ) -> Result<(), DomainError> {
        self.voice_sessions.lock().unwrap().push((
            g.into(),
            u.into(),
            n.into(),
            c.into(),
            cn.into(),
            d,
        ));
        Ok(())
    }
    async fn get_guild_voice_stats(
        &self,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<Vec<VoiceSessionStats>, DomainError> {
        Ok(self.voice_stats_returns.lock().unwrap().clone())
    }
    async fn count_unique_voice_users(&self, _: &str, _: u32) -> Result<i64, DomainError> {
        Ok(*self.unique_voice_users.lock().unwrap())
    }
}

// ── MockInfractionRepo ──

#[derive(Default)]
struct MockInfractionRepo {
    infractions: Mutex<Vec<Infraction>>,
}
fn sample_inf(action: Action) -> Infraction {
    Infraction {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        channel_id: "c".into(),
        user_id: "u".into(),
        username: "u".into(),
        display_name: None,
        message_id: "m".into(),
        content: "".into(),
        flags: DetectionFlags {
            spam: false,
            insult: false,
            profanity: false,
            link: false,
            phishing: false,
        },
        score: 0.0,
        action,
        reason: "".into(),
        duration: None,
        created_at: Utc::now(),
    }
}
#[async_trait]
impl InfractionRepository for MockInfractionRepo {
    async fn count_by_action_for_user(
        &self,
        _: &str,
        _: &str,
    ) -> Result<Vec<(String, u64)>, crate::domain::errors::DomainError> {
        Ok(vec![])
    }
    async fn save(&self, _: &Infraction) -> Result<(), DomainError> {
        Ok(())
    }
    async fn find_by_guild(
        &self,
        _: &str,
        _: &InfractionFilters,
    ) -> Result<Vec<Infraction>, DomainError> {
        Ok(self.infractions.lock().unwrap().clone())
    }
    async fn find_all(&self, _: i64, _: i64) -> Result<Vec<Infraction>, DomainError> {
        Ok(vec![])
    }
    async fn count_today(&self) -> Result<u64, DomainError> {
        Ok(7)
    }
    async fn find_by_id(&self, _: &str) -> Result<Option<Infraction>, DomainError> {
        Ok(None)
    }
    async fn delete_by_id(&self, _: &str) -> Result<bool, DomainError> {
        Ok(false)
    }
    async fn delete_older_than_days(&self, _: &str, _: i32) -> Result<u64, DomainError> {
        Ok(0)
    }
}

// ── MockCache ──

#[derive(Default)]
struct MockCache {
    invalidations: Mutex<Vec<String>>,
    json_store: Mutex<std::collections::HashMap<String, String>>,
}
#[async_trait]
impl CachePort for MockCache {
    async fn get_rules(&self, _: &str) -> Result<Option<Vec<Rule>>, DomainError> {
        Ok(None)
    }
    async fn set_rules(&self, _: &str, _: &[Rule]) -> Result<(), DomainError> {
        Ok(())
    }
    async fn invalidate_rules(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
    async fn get_json(&self, key: &str) -> Result<Option<String>, DomainError> {
        Ok(self.json_store.lock().unwrap().get(key).cloned())
    }
    async fn set_json(&self, key: &str, json: &str, _: u64) -> Result<(), DomainError> {
        self.json_store
            .lock()
            .unwrap()
            .insert(key.into(), json.into());
        Ok(())
    }
    async fn invalidate(&self, key: &str) -> Result<(), DomainError> {
        self.invalidations.lock().unwrap().push(key.into());
        self.json_store.lock().unwrap().remove(key);
        Ok(())
    }
    async fn invalidate_pattern(&self, _: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

struct MockServiceRegistry;
#[async_trait::async_trait]
impl crate::ports::outbound::ops::service_registry::ServiceRegistry for MockServiceRegistry {
    async fn count_services(
        &self,
    ) -> crate::ports::outbound::ops::service_registry::ServiceCounts {
        crate::ports::outbound::ops::service_registry::ServiceCounts {
            bots_online: 0,
            bots_total: 0,
            workers_online: 0,
            workers_total: 0,
        }
    }
    async fn ping(&self) -> bool {
        true
    }
}

fn make_service(
    stats: Arc<MockStatsRepo>,
    inf: Arc<MockInfractionRepo>,
    cache: Arc<MockCache>,
) -> ManageStatsService {
    ManageStatsService::new(stats, inf, cache, Arc::new(MockServiceRegistry))
}

// ══════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════

#[tokio::test]
async fn record_messages_increments_and_invalidates_overview() {
    let s = Arc::new(MockStatsRepo::default());
    let c = Arc::new(MockCache::default());
    let svc = make_service(
        s.clone(),
        Arc::new(MockInfractionRepo::default()),
        c.clone(),
    );
    svc.record_messages(RecordMessagesCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "Alice".into(),
        count: 5,
    })
    .await
    .unwrap();
    assert_eq!(
        s.inc_messages.lock().unwrap()[0],
        ("g1".into(), "u1".into(), "Alice".into(), 5)
    );
    assert_eq!(c.invalidations.lock().unwrap()[0], "stats:overview:g1");
}

#[tokio::test]
async fn record_voice_adds_seconds_saves_session_and_invalidates_all_periods() {
    let s = Arc::new(MockStatsRepo::default());
    let c = Arc::new(MockCache::default());
    let svc = make_service(
        s.clone(),
        Arc::new(MockInfractionRepo::default()),
        c.clone(),
    );
    svc.record_voice(RecordVoiceCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "Alice".into(),
        channel_id: "chan".into(),
        channel_name: "general".into(),
        seconds: 120,
    })
    .await
    .unwrap();
    assert_eq!(s.add_voice.lock().unwrap()[0].3, 120);
    assert_eq!(s.voice_sessions.lock().unwrap()[0].5, 120);
    let inv = c.invalidations.lock().unwrap();
    // 1 overview + 3 periodes (7, 30, 90)
    assert!(inv.contains(&"stats:overview:g1".to_string()));
    assert!(inv.contains(&"voice_stats:g1:7:20".to_string()));
    assert!(inv.contains(&"voice_stats:g1:30:20".to_string()));
    assert!(inv.contains(&"voice_stats:g1:90:20".to_string()));
}

#[tokio::test]
async fn record_voice_empty_channel_skips_session_save() {
    let s = Arc::new(MockStatsRepo::default());
    let svc = make_service(
        s.clone(),
        Arc::new(MockInfractionRepo::default()),
        Arc::new(MockCache::default()),
    );
    svc.record_voice(RecordVoiceCommand {
        guild_id: "g1".into(),
        user_id: "u1".into(),
        username: "Alice".into(),
        channel_id: "".into(),
        channel_name: "".into(),
        seconds: 60,
    })
    .await
    .unwrap();
    assert_eq!(s.add_voice.lock().unwrap()[0].3, 60);
    assert!(s.voice_sessions.lock().unwrap().is_empty());
}

#[tokio::test]
async fn get_user_stats_forwards() {
    let s = Arc::new(MockStatsRepo::default());
    *s.find_by_user_returns.lock().unwrap() = Some(sample_stats("g", "u", 10, 50));
    let svc = make_service(
        s,
        Arc::new(MockInfractionRepo::default()),
        Arc::new(MockCache::default()),
    );
    let got = svc.get_user_stats("g", "u").await.unwrap().unwrap();
    assert_eq!(got.message_count, 10);
}

#[tokio::test]
async fn get_guild_overview_aggregates_and_counts_infractions_by_action() {
    let s = Arc::new(MockStatsRepo::default());
    *s.find_by_guild_returns.lock().unwrap() = vec![
        sample_stats("g", "u1", 10, 100),
        sample_stats("g", "u2", 20, 200),
    ];
    let inf = Arc::new(MockInfractionRepo::default());
    *inf.infractions.lock().unwrap() = vec![
        sample_inf(Action::Warn),
        sample_inf(Action::Warn),
        sample_inf(Action::Mute),
        sample_inf(Action::Ban),
    ];
    let svc = make_service(s, inf, Arc::new(MockCache::default()));
    let ov = svc.get_guild_overview("g").await.unwrap();
    assert_eq!(ov.guild_id.as_str(), "g");
    assert_eq!(ov.total_messages, 30);
    assert_eq!(ov.total_voice_seconds, 300);
    assert_eq!(ov.active_members, 2);
    assert_eq!(ov.total_infractions, 4);
    assert_eq!(ov.total_warns, 2);
    assert_eq!(ov.total_mutes, 1);
    assert_eq!(ov.total_bans, 1);
}

#[tokio::test]
async fn get_guild_overview_is_cached() {
    let s = Arc::new(MockStatsRepo::default());
    let c = Arc::new(MockCache::default());
    let svc = make_service(
        s.clone(),
        Arc::new(MockInfractionRepo::default()),
        c.clone(),
    );
    let _ = svc.get_guild_overview("g").await.unwrap();
    let _ = svc.get_guild_overview("g").await.unwrap();
    // 2 appels au service, 1 seul en DB grace au cache.
    assert!(c
        .json_store
        .lock()
        .unwrap()
        .contains_key("stats:overview:g"));
}

#[tokio::test]
async fn get_leaderboard_forwards_to_repo() {
    let s = Arc::new(MockStatsRepo::default());
    *s.find_by_guild_returns.lock().unwrap() = vec![sample_stats("g", "u1", 1, 1)];
    let svc = make_service(
        s,
        Arc::new(MockInfractionRepo::default()),
        Arc::new(MockCache::default()),
    );
    assert_eq!(svc.get_leaderboard("g", 10).await.unwrap().len(), 1);
}

#[tokio::test]
async fn get_guild_voice_stats_aggregates_channels() {
    let s = Arc::new(MockStatsRepo::default());
    *s.voice_stats_returns.lock().unwrap() = vec![
        VoiceSessionStats {
            channel_id: "c1".into(),
            channel_name: "n1".into(),
            is_temporary: false,
            total_sessions: 10,
            total_duration_secs: 600,
            unique_users: 2,
            avg_duration_secs: 60,
            last_activity: None,
        },
        VoiceSessionStats {
            channel_id: "c2".into(),
            channel_name: "n2".into(),
            is_temporary: true,
            total_sessions: 5,
            total_duration_secs: 200,
            unique_users: 1,
            avg_duration_secs: 40,
            last_activity: None,
        },
    ];
    *s.unique_voice_users.lock().unwrap() = 3;
    let svc = make_service(
        s,
        Arc::new(MockInfractionRepo::default()),
        Arc::new(MockCache::default()),
    );
    let vs = svc.get_guild_voice_stats("g", 7, 20).await.unwrap();
    assert_eq!(vs.total_channels, 2);
    assert_eq!(vs.total_sessions, 15);
    assert_eq!(vs.total_duration_secs, 800);
    assert_eq!(vs.avg_session_secs, 800 / 15);
    assert_eq!(vs.temp_channels, 1);
    assert_eq!(vs.perm_channels, 1);
    assert_eq!(vs.unique_users, 3);
}

#[tokio::test]
async fn get_guild_voice_stats_handles_zero_sessions() {
    // Avg doit etre 0 si total_sessions == 0 (garde-fou div-zero).
    let s = Arc::new(MockStatsRepo::default());
    *s.voice_stats_returns.lock().unwrap() = vec![];
    let svc = make_service(
        s,
        Arc::new(MockInfractionRepo::default()),
        Arc::new(MockCache::default()),
    );
    let vs = svc.get_guild_voice_stats("g", 7, 20).await.unwrap();
    assert_eq!(vs.total_sessions, 0);
    assert_eq!(vs.avg_session_secs, 0);
}
