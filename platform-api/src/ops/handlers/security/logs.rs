//! Agregations sur les logs API : top IPs, echecs d'auth, tendance de trafic.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::ops::{ApiError, AppState};
use platform_core::ops::domain::entities::security_log::LogWindow;

// ── Top IPs par requetes ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct WindowQuery {
    /// "1h" / "24h" / "7d", defaut 1h
    pub window: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct TopIpEntry {
    pub client_ip: String,
    pub total: i64,
    pub failed: i64,
    pub last_seen: String,
}

pub async fn top_ips(
    State(state): State<AppState>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<TopIpEntry>>, ApiError> {
    let window = LogWindow::parse(q.window.as_deref().unwrap_or("1h"));
    let limit = crate::ops::handlers::normalize_in(q.limit, 20, 1, 100);

    let rows = state.security_logs_uc.top_ips(window, limit).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| TopIpEntry {
                client_ip: r.client_ip,
                total: r.total,
                failed: r.failed,
                last_seen: r.last_seen.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            })
            .collect(),
    ))
}

// ── Echecs d'auth (401/403) recents ─────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct AuthFailureEntry {
    pub timestamp: String,
    pub status_code: i64,
    pub method: String,
    pub route: String,
    pub client_ip: String,
    pub user_agent: String,
}

pub async fn auth_failures(
    State(state): State<AppState>,
    Query(q): Query<WindowQuery>,
) -> Result<Json<Vec<AuthFailureEntry>>, ApiError> {
    let window = LogWindow::parse(q.window.as_deref().unwrap_or("24h"));
    let limit = crate::ops::handlers::normalize_in(q.limit, 100, 1, 500);

    let rows = state.security_logs_uc.auth_failures(window, limit).await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| AuthFailureEntry {
                timestamp: r.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                status_code: r.status_code,
                method: r.method,
                route: r.route,
                client_ip: r.client_ip,
                user_agent: r.user_agent,
            })
            .collect(),
    ))
}

// ── Trafic anormal : graphe req/s sur N heures ──────────────────────────

#[derive(Debug, Deserialize)]
pub struct TrafficTrendQuery {
    /// Fenetre : "1h", "6h", "24h", "7d"
    pub window: Option<String>,
    /// Bucket : taille en minutes (5 par defaut)
    pub bucket_minutes: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct TrafficDatapoint {
    pub timestamp: String,
    pub total: i64,
    pub errors: i64, // 4xx + 5xx
}

#[derive(Debug, Serialize)]
pub struct TrafficTrendResponse {
    pub datapoints: Vec<TrafficDatapoint>,
    pub baseline_avg: f64,
    pub peak: i64,
    pub peak_at: Option<String>,
    pub alert: bool,
    pub alert_reason: Option<String>,
}

pub async fn traffic_trend(
    State(state): State<AppState>,
    Query(q): Query<TrafficTrendQuery>,
) -> Result<Json<TrafficTrendResponse>, ApiError> {
    let window = LogWindow::parse(q.window.as_deref().unwrap_or("24h"));
    let bucket_min = crate::ops::handlers::normalize_in(q.bucket_minutes.map(i64::from), 5, 1, 60);

    let trend = state
        .security_logs_uc
        .traffic_trend(window, bucket_min)
        .await?;

    Ok(Json(TrafficTrendResponse {
        datapoints: trend
            .datapoints
            .into_iter()
            .map(|d| TrafficDatapoint {
                timestamp: d.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                total: d.total,
                errors: d.errors,
            })
            .collect(),
        baseline_avg: trend.baseline_avg,
        peak: trend.peak,
        peak_at: trend
            .peak_at
            .map(|t| t.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
        alert: trend.alert,
        alert_reason: trend.alert_reason,
    }))
}
