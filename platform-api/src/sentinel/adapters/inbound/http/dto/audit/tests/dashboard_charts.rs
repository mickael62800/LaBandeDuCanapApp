use super::*;
use chrono::NaiveDate;
use platform_core::sentinel::domain::entities::community::daily_activity::DailyActivity;
use uuid::Uuid;

fn sample(messages: i64, warns: i32) -> DailyActivity {
    DailyActivity {
        id: Uuid::new_v4(),
        guild_id: "g".into(),
        day: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
        messages,
        voice_minutes: 42,
        active_members: 10,
        new_members: 3,
        leaves: 1,
        infractions: 5,
        warns,
        mutes: 2,
        bans: 1,
    }
}

#[test]
fn daily_activity_dto_copies_all_counters() {
    let a = sample(1000, 5);
    let dto: DailyActivityDto = a.into();
    assert_eq!(dto.messages, 1000);
    assert_eq!(dto.voice_minutes, 42);
    assert_eq!(dto.active_members, 10);
    assert_eq!(dto.new_members, 3);
    assert_eq!(dto.leaves, 1);
    assert_eq!(dto.infractions, 5);
    assert_eq!(dto.warns, 5);
    assert_eq!(dto.mutes, 2);
    assert_eq!(dto.bans, 1);
}

#[test]
fn daily_activity_dto_day_is_iso_format() {
    let dto: DailyActivityDto = sample(0, 0).into();
    assert_eq!(dto.day, "2026-01-15");
}

#[test]
fn daily_activity_dto_zero_values() {
    let dto: DailyActivityDto = sample(0, 0).into();
    assert_eq!(dto.messages, 0);
    assert_eq!(dto.warns, 0);
}
