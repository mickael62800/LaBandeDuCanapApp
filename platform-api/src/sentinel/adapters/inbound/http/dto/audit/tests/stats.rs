use super::*;
use chrono::Utc;
use platform_core::sentinel::domain::entities::audit::user_stats::GuildStatsOverview;
use platform_core::sentinel::domain::entities::audit::user_stats::GuildVoiceStats;
use platform_core::sentinel::domain::entities::audit::user_stats::UserStats;
use platform_core::sentinel::domain::entities::audit::user_stats::VoiceSessionStats;
use uuid::Uuid;

fn user_stats(voice_seconds: u64) -> UserStats {
    UserStats {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        message_count: 100,
        voice_seconds,
        updated_at: Utc::now(),
    }
}

fn voice_session(total_secs: i64) -> VoiceSessionStats {
    VoiceSessionStats {
        channel_id: "c".into(),
        channel_name: "Voice".into(),
        is_temporary: false,
        total_sessions: 10,
        total_duration_secs: total_secs,
        unique_users: 5,
        avg_duration_secs: total_secs / 10,
        last_activity: Some(Utc::now()),
    }
}

// ── UserStatsDto.voice_hours ──

#[test]
fn user_stats_dto_zero_seconds_zero_hours() {
    let dto: UserStatsDto = user_stats(0).into();
    assert_eq!(dto.voice_hours, 0.0);
    assert_eq!(dto.voice_seconds, 0);
}

#[test]
fn user_stats_dto_one_hour() {
    let dto: UserStatsDto = user_stats(3600).into();
    assert!((dto.voice_hours - 1.0).abs() < 1e-9);
}

#[test]
fn user_stats_dto_fraction_of_hour() {
    let dto: UserStatsDto = user_stats(1800).into();
    assert!((dto.voice_hours - 0.5).abs() < 1e-9);
}

#[test]
fn user_stats_dto_large_duration() {
    let dto: UserStatsDto = user_stats(360_000).into(); // 100h
    assert!((dto.voice_hours - 100.0).abs() < 1e-6);
}

#[test]
fn user_stats_dto_formats_date_rfc3339() {
    let dto: UserStatsDto = user_stats(0).into();
    assert!(dto.updated_at.contains('T'));
}

// ── VoiceSessionStatsDto.total_duration_hours ──

#[test]
fn voice_session_dto_converts_to_hours() {
    let dto: VoiceSessionStatsDto = voice_session(7200).into();
    assert!((dto.total_duration_hours - 2.0).abs() < 1e-9);
    assert_eq!(dto.total_duration_secs, 7200);
}

#[test]
fn voice_session_dto_zero_total_secs() {
    let dto: VoiceSessionStatsDto = voice_session(0).into();
    assert_eq!(dto.total_duration_hours, 0.0);
}

#[test]
fn voice_session_dto_preserves_last_activity_rfc3339() {
    let dto: VoiceSessionStatsDto = voice_session(100).into();
    assert!(dto.last_activity.unwrap().contains('T'));
}

#[test]
fn voice_session_dto_none_last_activity() {
    let mut s = voice_session(100);
    s.last_activity = None;
    let dto: VoiceSessionStatsDto = s.into();
    assert!(dto.last_activity.is_none());
}

// ── GuildVoiceStatsDto ──

#[test]
fn guild_voice_stats_dto_maps_channels() {
    let g = GuildVoiceStats {
        total_channels: 3,
        total_sessions: 30,
        total_duration_secs: 3600,
        unique_users: 15,
        avg_session_secs: 120,
        temp_channels: 1,
        perm_channels: 2,
        channels: vec![voice_session(1200), voice_session(2400)],
    };
    let dto: GuildVoiceStatsDto = g.into();
    assert_eq!(dto.channels.len(), 2);
    assert!((dto.total_duration_hours - 1.0).abs() < 1e-9);
    assert_eq!(dto.temp_channels + dto.perm_channels, dto.total_channels);
}

// ── GuildOverviewDto ──

#[test]
fn guild_overview_dto_aggregates() {
    let o = GuildStatsOverview {
        guild_id: "g".into(),
        total_messages: 1000,
        total_voice_seconds: 36000, // 10h
        active_members: 50,
        total_infractions: 10,
        total_warns: 5,
        total_mutes: 3,
        total_bans: 2,
        top_members: vec![user_stats(3600), user_stats(7200)],
    };
    let dto: GuildOverviewDto = o.into();
    assert!((dto.total_voice_hours - 10.0).abs() < 1e-9);
    assert_eq!(dto.top_members.len(), 2);
    assert_eq!(dto.total_warns + dto.total_mutes + dto.total_bans, 10);
}

// ── Command conversions ──

#[test]
fn record_messages_dto_to_command() {
    let dto = RecordMessagesDto {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        count: 42,
    };
    let cmd: RecordMessagesCommand = dto.into();
    assert_eq!(cmd.count, 42);
}

#[test]
fn record_voice_dto_to_command() {
    let dto = RecordVoiceDto {
        guild_id: "g".into(),
        user_id: "u".into(),
        username: "alice".into(),
        seconds: 600,
        channel_id: Some("c".into()),
        channel_name: "Voice".into(),
    };
    let cmd: RecordVoiceCommand = dto.into();
    assert_eq!(cmd.seconds, 600);
    assert_eq!(cmd.channel_id, "c".into());
}
