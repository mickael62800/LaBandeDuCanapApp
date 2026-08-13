use super::*;

use chrono::TimeZone;
use uuid::Uuid;

fn ts() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap()
}

fn sample_user_stats() -> UserStats {
    UserStats {
        id: Uuid::nil(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        message_count: 1500,
        voice_seconds: 7200,
        updated_at: ts(),
    }
}

#[test]
fn user_stats_to_proto_full_mapping() {
    let p = user_stats_to_proto(sample_user_stats());
    assert_eq!(p.user_id, "u");
    assert_eq!(p.message_count, 1500);
    assert_eq!(p.voice_seconds, 7200);
    assert_eq!(p.updated_at, ts().to_rfc3339());
}

#[test]
fn guild_overview_to_proto_full_mapping() {
    let o = GuildStatsOverview {
        guild_id: "g1".into(),
        total_messages: 50000,
        total_voice_seconds: 360000,
        active_members: 200,
        total_infractions: 30,
        total_warns: 20,
        total_mutes: 8,
        total_bans: 2,
        top_members: vec![
            sample_user_stats(),
            sample_user_stats(),
            sample_user_stats(),
        ],
    };
    let p = guild_overview_to_proto(o);
    assert_eq!(p.guild_id, "g1");
    assert_eq!(p.total_messages, 50000);
    assert_eq!(p.total_voice_seconds, 360000);
    assert_eq!(p.active_members, 200);
    assert_eq!(p.total_warns + p.total_mutes + p.total_bans, 30);
    assert_eq!(p.top_members.len(), 3);
}

#[test]
fn guild_overview_to_proto_empty_top_members() {
    let o = GuildStatsOverview {
        guild_id: "g".into(),
        total_messages: 0,
        total_voice_seconds: 0,
        active_members: 0,
        total_infractions: 0,
        total_warns: 0,
        total_mutes: 0,
        total_bans: 0,
        top_members: vec![],
    };
    let p = guild_overview_to_proto(o);
    assert!(p.top_members.is_empty());
}

// ── RPC handler tests avec mock ──

use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use async_trait::async_trait;
use chrono::Utc;
use platform_core::sentinel::domain::entities::audit::dashboard_stats::DashboardStats;
use platform_core::sentinel::domain::entities::audit::user_stats::GuildVoiceStats;
use platform_core::sentinel::domain::errors::DomainError;
use platform_core::sentinel::ports::inbound::audit::manage_stats::ManageStatsUseCase;
use platform_core::sentinel::ports::inbound::audit::manage_stats::RecordMessagesCommand;
use platform_core::sentinel::ports::inbound::audit::manage_stats::RecordVoiceCommand;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Default)]
struct MockStatsUc {
    record_msg_calls: Mutex<Vec<RecordMessagesCommand>>,
    record_voice_calls: Mutex<Vec<RecordVoiceCommand>>,
    user_stats_return: Mutex<Option<UserStats>>,
    overview_return: Mutex<Option<GuildStatsOverview>>,
    leaderboard_return: Mutex<Vec<UserStats>>,
    leaderboard_calls: Mutex<Vec<u32>>,
}

#[async_trait]
impl ManageStatsUseCase for MockStatsUc {
    async fn record_messages(&self, cmd: RecordMessagesCommand) -> Result<(), DomainError> {
        self.record_msg_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn record_voice(&self, cmd: RecordVoiceCommand) -> Result<(), DomainError> {
        self.record_voice_calls.lock().unwrap().push(cmd);
        Ok(())
    }
    async fn get_user_stats(&self, _: &str, _: &str) -> Result<Option<UserStats>, DomainError> {
        Ok(self.user_stats_return.lock().unwrap().clone())
    }
    async fn get_guild_overview(&self, g: &str) -> Result<GuildStatsOverview, DomainError> {
        Ok(self
            .overview_return
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(GuildStatsOverview {
                guild_id: g.into(),
                total_messages: 0,
                total_voice_seconds: 0,
                active_members: 0,
                total_infractions: 0,
                total_warns: 0,
                total_mutes: 0,
                total_bans: 0,
                top_members: vec![],
            }))
    }
    async fn get_leaderboard(&self, _: &str, limit: u32) -> Result<Vec<UserStats>, DomainError> {
        self.leaderboard_calls.lock().unwrap().push(limit);
        Ok(self.leaderboard_return.lock().unwrap().clone())
    }
    async fn get_dashboard_stats(&self) -> Result<DashboardStats, DomainError> {
        unimplemented!()
    }
    async fn get_guild_voice_stats(
        &self,
        _: &str,
        _: u32,
        _: u32,
    ) -> Result<GuildVoiceStats, DomainError> {
        unimplemented!()
    }
}

fn grpc(uc: Arc<MockStatsUc>) -> StatsGrpc {
    StatsGrpc {
        stats_uc: uc,
        broadcaster: Arc::new(EventBroadcaster::new()),
    }
}

fn sample_user() -> UserStats {
    UserStats {
        id: uuid::Uuid::nil(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        message_count: 100,
        voice_seconds: 3600,
        updated_at: Utc::now(),
    }
}

#[tokio::test]
async fn record_messages_delegates_to_uc() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .record_messages(Request::new(proto::RecordMessagesRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "a".into(),
            count: 50,
        }))
        .await
        .unwrap();
    let calls = uc.record_msg_calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].count, 50);
}

#[tokio::test]
async fn record_voice_delegates_to_uc() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .record_voice(Request::new(proto::RecordVoiceRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
            username: "a".into(),
            seconds: 1800,
            channel_id: "c".into(),
            channel_name: "General".into(),
        }))
        .await
        .unwrap();
    assert_eq!(uc.record_voice_calls.lock().unwrap()[0].seconds, 1800);
}

#[tokio::test]
async fn get_user_stats_returns_some_when_uc_has_data() {
    let uc = Arc::new(MockStatsUc::default());
    *uc.user_stats_return.lock().unwrap() = Some(sample_user());
    let g = grpc(uc);
    let resp = g
        .get_user_stats(Request::new(proto::GetUserStatsRequest {
            guild_id: "g".into(),
            user_id: "u".into(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().stats.is_some());
}

#[tokio::test]
async fn get_user_stats_returns_none_when_uc_empty() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc);
    let resp = g
        .get_user_stats(Request::new(proto::GetUserStatsRequest {
            guild_id: "g".into(),
            user_id: "ghost".into(),
        }))
        .await
        .unwrap();
    assert!(resp.into_inner().stats.is_none());
}

#[tokio::test]
async fn get_guild_overview_delegates() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc);
    let resp = g
        .get_guild_overview(Request::new(proto::GetGuildOverviewRequest {
            guild_id: "g1".into(),
        }))
        .await
        .unwrap();
    assert_eq!(resp.into_inner().guild_id, "g1");
}

#[tokio::test]
async fn get_leaderboard_default_limit_when_zero() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .get_leaderboard(Request::new(proto::GetLeaderboardRequest {
            guild_id: "g".into(),
            limit: 0,
        }))
        .await
        .unwrap();
    let calls = uc.leaderboard_calls.lock().unwrap();
    // limit=0 → fallback 10
    assert_eq!(calls[0], 10);
}

#[tokio::test]
async fn get_leaderboard_caps_at_50() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .get_leaderboard(Request::new(proto::GetLeaderboardRequest {
            guild_id: "g".into(),
            limit: 1000,
        }))
        .await
        .unwrap();
    let calls = uc.leaderboard_calls.lock().unwrap();
    // cap a 50
    assert_eq!(calls[0], 50);
}

#[tokio::test]
async fn get_leaderboard_preserves_valid_limit() {
    let uc = Arc::new(MockStatsUc::default());
    let g = grpc(uc.clone());
    let _ = g
        .get_leaderboard(Request::new(proto::GetLeaderboardRequest {
            guild_id: "g".into(),
            limit: 25,
        }))
        .await
        .unwrap();
    let calls = uc.leaderboard_calls.lock().unwrap();
    assert_eq!(calls[0], 25);
}
