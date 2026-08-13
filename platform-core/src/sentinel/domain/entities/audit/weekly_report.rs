/// Rapport d'activite hebdomadaire d'un serveur, agrege server-side depuis les
/// events d'audit deja persistes (table `audit_logs`) sur une fenetre de 7 jours.
///
/// Remonte l'ancien `WeeklyTracker`/`WeeklyStats` du bot (agregation RAM morte)
/// vers un calcul deterministe cote serveur : on compte les rows par `event_type`
/// et on les mappe vers les compteurs metier ci-dessous.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WeeklyReport {
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

/// Fenetre d'agregation du rapport hebdomadaire, en jours.
pub const WEEKLY_REPORT_WINDOW_DAYS: u32 = 7;

impl WeeklyReport {
    /// Construit un rapport a partir des comptes bruts par `event_type` (tels que
    /// renvoyes par le port de comptage). Le mapping event_type -> compteur est la
    /// regle metier : il reflete les `event_type` emis par le bot d'audit.
    pub fn from_event_counts<I, S>(counts: I) -> Self
    where
        I: IntoIterator<Item = (S, u64)>,
        S: AsRef<str>,
    {
        let mut report = WeeklyReport::default();
        for (event_type, count) in counts {
            match event_type.as_ref() {
                "member_join" => report.member_joins += count,
                "member_leave" => report.member_leaves += count,
                "member_ban" => report.bans += count,
                "message_delete" | "message_delete_bulk" => report.messages_deleted += count,
                "message_edit" => report.messages_edited += count,
                "role_create" | "role_delete" | "role_update" => report.role_changes += count,
                "channel_create" | "channel_delete" => report.channel_changes += count,
                "voice_join" | "voice_leave" | "voice_move" => report.voice_events += count,
                "anomaly_detected" => report.anomalies += count,
                _ => {}
            }
        }
        report
    }
}

#[cfg(test)]
#[path = "tests/weekly_report.rs"]
mod tests;
