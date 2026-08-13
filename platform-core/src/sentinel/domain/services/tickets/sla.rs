use std::time::{Duration, Instant};

use dashmap::DashMap;

/// Tracker SLA pour les tickets.
///
/// Le bot ne fait que mesurer le delai de premiere reponse staff pour le
/// remonter a l'API (`api.update_ticket_sla`). La decision d'escalade/breach
/// vit dans les workers API (events Redis `ticket_sla_*`).
pub struct SlaTracker {
    /// ticket_id -> timestamp de creation
    created: DashMap<String, Instant>,
    /// ticket_id -> timestamp de premiere reponse staff
    first_response: DashMap<String, Instant>,
}

impl SlaTracker {
    pub fn new() -> Self {
        Self {
            created: DashMap::new(),
            first_response: DashMap::new(),
        }
    }

    pub fn record_creation(&self, ticket_id: &str) {
        self.created.insert(ticket_id.to_string(), Instant::now());
    }

    pub fn record_staff_response(&self, ticket_id: &str) -> Option<Duration> {
        if self.first_response.contains_key(ticket_id) {
            return None;
        }

        let created_at = self.created.get(ticket_id)?;
        let now = Instant::now();
        let duration = now.duration_since(*created_at);

        self.first_response.insert(ticket_id.to_string(), now);
        Some(duration)
    }
}

impl Default for SlaTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ── Décision d'escalade/warning SLA (consommée par sentinel-worker) ──

pub const DEFAULT_SLA_FIRST_RESPONSE_MINUTES: i64 = 30;
pub const DEFAULT_SLA_ESCALATION_MINUTES: i64 = 60;

/// Nombre de jours d'inactivite avant fermeture automatique d'un ticket
/// (override par guild via `bot_guild_config.inactive_close_days`).
pub const DEFAULT_INACTIVE_CLOSE_DAYS: i64 = 7;

/// Resolution d'un seuil SLA : la valeur configuree par la guild est prise
/// BRUTE si presente (un seuil <= 0 signifie « desactive », tranche par
/// [`is_breached`]) ; cle absente ou non numerique = defaut.
pub fn effective_threshold(configured: Option<i64>, default: i64) -> i64 {
    configured.unwrap_or(default)
}

/// Décision de breach SLA : le ticket a dépassé le seuil. Un seuil <= 0
/// signifie « désactivé » — jamais de breach.
pub fn is_breached(age_minutes: i64, threshold_minutes: i64) -> bool {
    threshold_minutes > 0 && age_minutes >= threshold_minutes
}

/// Formate une duree en texte lisible.
pub fn format_sla_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let minutes = total_secs / 60;
    let hours = minutes / 60;

    if hours > 0 {
        let remaining_min = minutes % 60;
        if remaining_min > 0 {
            format!("{}h{}min", hours, remaining_min)
        } else {
            format!("{}h", hours)
        }
    } else if minutes > 0 {
        format!("{}min", minutes)
    } else {
        format!("{}s", total_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_response_measured_once() {
        let t = SlaTracker::new();
        t.record_creation("T1");
        assert!(t.record_staff_response("T1").is_some());
        // Idempotent : la 2e reponse staff ne remesure pas.
        assert!(t.record_staff_response("T1").is_none());
    }

    #[test]
    fn response_without_creation_ignored() {
        let t = SlaTracker::new();
        assert!(t.record_staff_response("inconnu").is_none());
    }

    #[test]
    fn tickets_independent() {
        let t = SlaTracker::new();
        t.record_creation("T1");
        t.record_creation("T2");
        assert!(t.record_staff_response("T1").is_some());
        assert!(t.record_staff_response("T2").is_some());
    }

    #[test]
    fn breach_at_and_after_threshold() {
        assert!(!is_breached(59, 60));
        assert!(is_breached(60, 60));
        assert!(is_breached(120, 60));
    }

    #[test]
    fn breach_disabled_threshold_never_fires() {
        assert!(!is_breached(10_000, 0));
        assert!(!is_breached(10_000, -1));
    }

    #[test]
    fn effective_threshold_configured_taken_raw() {
        assert_eq!(effective_threshold(Some(15), 60), 15);
        // <= 0 = desactive : conserve tel quel, c'est is_breached qui tranche.
        assert_eq!(effective_threshold(Some(0), 60), 0);
        assert_eq!(effective_threshold(Some(-5), 60), -5);
    }

    #[test]
    fn effective_threshold_absent_falls_back_to_default() {
        assert_eq!(
            effective_threshold(None, DEFAULT_SLA_ESCALATION_MINUTES),
            60
        );
        assert_eq!(effective_threshold(None, DEFAULT_INACTIVE_CLOSE_DAYS), 7);
    }

    #[test]
    fn format_seconds() {
        assert_eq!(format_sla_duration(Duration::from_secs(42)), "42s");
    }

    #[test]
    fn format_minutes() {
        assert_eq!(format_sla_duration(Duration::from_secs(180)), "3min");
    }

    #[test]
    fn format_hours_exact() {
        assert_eq!(format_sla_duration(Duration::from_secs(7200)), "2h");
    }

    #[test]
    fn format_hours_and_minutes() {
        assert_eq!(format_sla_duration(Duration::from_secs(3660)), "1h1min");
    }
}
