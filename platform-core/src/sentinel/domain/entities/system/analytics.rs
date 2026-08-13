use crate::sentinel::domain::entities::system::discord_ids::UserId;

/// Activite par heure — pour heatmaps.
#[derive(Debug, Clone)]
pub struct HourlyActivity {
    pub hour: i16,
    pub day_of_week: i16, // 0=lundi, 6=dimanche
    pub messages: i64,
    pub infractions: i32,
}

/// Distribution des actions de moderation.
#[derive(Debug, Clone)]
pub struct ActionDistribution {
    pub action: String,
    pub count: i64,
    pub percentage: f64,
}

/// Top infracteur.
#[derive(Debug, Clone)]
pub struct TopInfractor {
    pub user_id: UserId,
    pub username: String,
    pub total_infractions: i64,
    pub warns: i64,
    pub deletes: i64,
    pub mutes: i64,
    pub bans: i64,
}

/// Trend de moderation par jour.
#[derive(Debug, Clone)]
pub struct ModerationTrend {
    pub day: chrono::NaiveDate,
    pub total: i64,
    pub warns: i64,
    pub deletes: i64,
    pub mutes: i64,
    pub bans: i64,
}

/// Pic d'activite — heure la plus active.
#[derive(Debug, Clone)]
pub struct PeakActivity {
    pub hour: i16,
    pub avg_messages: f64,
    pub avg_infractions: f64,
}
