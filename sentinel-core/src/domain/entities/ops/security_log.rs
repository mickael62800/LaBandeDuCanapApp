//! Entites d'analyse des logs API pour le panel securite (top IPs, echecs
//! d'auth, courbe de trafic). Alimentees par la table `logs` (categorie `api`).

use chrono::{DateTime, Utc};

/// Fenetre temporelle d'analyse. Le mapping vers un intervalle SQL est un
/// detail infra resolu par l'adapter outbound.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogWindow {
    OneHour,
    TwentyFourHours,
    SevenDays,
}

impl LogWindow {
    /// Parse "1h" / "24h" / "7d" (defaut : 1h).
    pub fn parse(s: &str) -> Self {
        match s {
            "24h" => LogWindow::TwentyFourHours,
            "7d" => LogWindow::SevenDays,
            _ => LogWindow::OneHour,
        }
    }
}

/// Une IP et son activite agregee sur la fenetre.
#[derive(Debug, Clone)]
pub struct TopIp {
    pub client_ip: String,
    pub total: i64,
    pub failed: i64,
    pub last_seen: DateTime<Utc>,
}

/// Un echec d'authentification (401/403).
#[derive(Debug, Clone)]
pub struct AuthFailure {
    pub timestamp: DateTime<Utc>,
    pub status_code: i64,
    pub method: String,
    pub route: String,
    pub client_ip: String,
    pub user_agent: String,
}

/// Un point de la courbe de trafic (bucket temporel).
#[derive(Debug, Clone)]
pub struct TrafficPoint {
    pub timestamp: DateTime<Utc>,
    pub total: i64,
    pub errors: i64,
}

/// Courbe de trafic + statistiques derivees (moyenne, pic, alerte).
#[derive(Debug, Clone)]
pub struct TrafficTrend {
    pub datapoints: Vec<TrafficPoint>,
    pub baseline_avg: f64,
    pub peak: i64,
    pub peak_at: Option<DateTime<Utc>>,
    pub alert: bool,
    pub alert_reason: Option<String>,
}

impl TrafficTrend {
    /// Calcule les stats a partir des points bruts. Alerte si un pic depasse
    /// 3x la moyenne (avec au moins 10 buckets pour que ce soit significatif).
    pub fn from_points(datapoints: Vec<TrafficPoint>) -> Self {
        let n = datapoints.len() as f64;
        let sum: i64 = datapoints.iter().map(|d| d.total).sum();
        let baseline_avg = if n > 0.0 { sum as f64 / n } else { 0.0 };
        let peak = datapoints.iter().map(|d| d.total).max().unwrap_or(0);
        let peak_at = datapoints
            .iter()
            .max_by_key(|d| d.total)
            .map(|d| d.timestamp);

        let alert =
            baseline_avg > 0.0 && datapoints.len() > 10 && (peak as f64) > baseline_avg * 3.0;
        let alert_reason = if alert {
            Some(format!(
                "Pic à {peak} req sur 1 bucket (3× moyenne {baseline_avg:.1})"
            ))
        } else {
            None
        };

        Self {
            datapoints,
            baseline_avg,
            peak,
            peak_at,
            alert,
            alert_reason,
        }
    }
}
