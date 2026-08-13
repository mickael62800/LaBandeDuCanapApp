use platform_core::sentinel::domain::entities::community::daily_activity::DailyActivity;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct ChartQueryParams {
    pub guild_id: Option<String>,
    pub days: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct DailyActivityDto {
    pub day: String,
    pub messages: i64,
    pub voice_minutes: i64,
    pub active_members: i32,
    pub new_members: i32,
    pub leaves: i32,
    pub infractions: i32,
    pub warns: i32,
    pub mutes: i32,
    pub bans: i32,
}

impl From<DailyActivity> for DailyActivityDto {
    fn from(a: DailyActivity) -> Self {
        Self {
            day: a.day.to_string(),
            messages: a.messages,
            voice_minutes: a.voice_minutes,
            active_members: a.active_members,
            new_members: a.new_members,
            leaves: a.leaves,
            infractions: a.infractions,
            warns: a.warns,
            mutes: a.mutes,
            bans: a.bans,
        }
    }
}

#[cfg(test)]
#[path = "tests/dashboard_charts.rs"]
mod tests;
