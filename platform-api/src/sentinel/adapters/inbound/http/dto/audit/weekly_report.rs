use serde::Serialize;

use platform_core::sentinel::domain::entities::audit::weekly_report::WeeklyReport;

/// Rapport hebdomadaire agrege server-side, renvoye au bot (qui rend l'embed) et
/// au frontend. Les compteurs couvrent une fenetre glissante de 7 jours.
#[derive(Debug, Serialize)]
pub struct WeeklyReportDto {
    pub member_joins: u64,
    pub member_leaves: u64,
    pub bans: u64,
    pub messages_deleted: u64,
    pub messages_edited: u64,
    pub role_changes: u64,
    pub channel_changes: u64,
    pub voice_events: u64,
    pub anomalies: u64,
}

impl From<WeeklyReport> for WeeklyReportDto {
    fn from(r: WeeklyReport) -> Self {
        Self {
            member_joins: r.member_joins,
            member_leaves: r.member_leaves,
            bans: r.bans,
            messages_deleted: r.messages_deleted,
            messages_edited: r.messages_edited,
            role_changes: r.role_changes,
            channel_changes: r.channel_changes,
            voice_events: r.voice_events,
            anomalies: r.anomalies,
        }
    }
}
