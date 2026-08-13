use platform_core::sentinel::domain::entities::system::analytics::*;
use platform_core::sentinel::domain::entities::system::discord_ids::UserId;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub guild_id: Option<String>,
    pub days: Option<i32>,
    pub limit: Option<i64>,
}

impl AnalyticsQuery {
    pub fn days(&self) -> i32 {
        crate::sentinel::adapters::inbound::http::helpers::normalize_in(self.days, 30, 1, 90)
    }

    pub fn limit(&self) -> i64 {
        crate::sentinel::adapters::inbound::http::helpers::normalize_in(self.limit, 10, 1, 50)
    }
}

// ── Heatmap ──

#[derive(Debug, Serialize, Deserialize)]
pub struct HeatmapPointDto {
    pub hour: i16,
    pub day_of_week: i16,
    pub day_name: String,
    pub messages: i64,
    pub infractions: i32,
}

impl From<HourlyActivity> for HeatmapPointDto {
    fn from(h: HourlyActivity) -> Self {
        Self {
            hour: h.hour,
            day_of_week: h.day_of_week,
            day_name: day_name(h.day_of_week).to_string(),
            messages: h.messages,
            infractions: h.infractions,
        }
    }
}

// ── Action distribution ──

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionDistributionDto {
    pub action: String,
    pub count: i64,
    pub percentage: f64,
}

impl From<ActionDistribution> for ActionDistributionDto {
    fn from(a: ActionDistribution) -> Self {
        Self {
            action: a.action,
            count: a.count,
            percentage: (a.percentage * 10.0).round() / 10.0,
        }
    }
}

// ── Top infracteurs ──

#[derive(Debug, Serialize, Deserialize)]
pub struct TopInfractorDto {
    pub user_id: UserId,
    pub username: String,
    pub total_infractions: i64,
    pub warns: i64,
    pub deletes: i64,
    pub mutes: i64,
    pub bans: i64,
}

impl From<TopInfractor> for TopInfractorDto {
    fn from(t: TopInfractor) -> Self {
        Self {
            user_id: t.user_id,
            username: t.username,
            total_infractions: t.total_infractions,
            warns: t.warns,
            deletes: t.deletes,
            mutes: t.mutes,
            bans: t.bans,
        }
    }
}

// ── Trend moderation ──

#[derive(Debug, Serialize, Deserialize)]
pub struct ModerationTrendDto {
    pub day: String,
    pub total: i64,
    pub warns: i64,
    pub deletes: i64,
    pub mutes: i64,
    pub bans: i64,
}

impl From<ModerationTrend> for ModerationTrendDto {
    fn from(t: ModerationTrend) -> Self {
        Self {
            day: t.day.to_string(),
            total: t.total,
            warns: t.warns,
            deletes: t.deletes,
            mutes: t.mutes,
            bans: t.bans,
        }
    }
}

// ── Peak hours ──

#[derive(Debug, Serialize, Deserialize)]
pub struct PeakHourDto {
    pub hour: i16,
    pub label: String,
    pub avg_messages: f64,
    pub avg_infractions: f64,
}

impl From<PeakActivity> for PeakHourDto {
    fn from(p: PeakActivity) -> Self {
        Self {
            label: format!("{:02}h-{:02}h", p.hour, (p.hour + 1) % 24),
            hour: p.hour,
            avg_messages: (p.avg_messages * 10.0).round() / 10.0,
            avg_infractions: (p.avg_infractions * 10.0).round() / 10.0,
        }
    }
}

// ── Reponse complete analytics ──

#[derive(Debug, Serialize, Deserialize)]
pub struct FullAnalyticsDto {
    pub heatmap: Vec<HeatmapPointDto>,
    pub action_distribution: Vec<ActionDistributionDto>,
    pub top_infractors: Vec<TopInfractorDto>,
    pub moderation_trend: Vec<ModerationTrendDto>,
    pub peak_hours: Vec<PeakHourDto>,
}

fn day_name(dow: i16) -> &'static str {
    match dow {
        0 => "Lundi",
        1 => "Mardi",
        2 => "Mercredi",
        3 => "Jeudi",
        4 => "Vendredi",
        5 => "Samedi",
        6 => "Dimanche",
        _ => "?",
    }
}

#[cfg(test)]
#[path = "tests/analytics.rs"]
mod tests;
