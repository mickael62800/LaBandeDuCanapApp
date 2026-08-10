//! 6 jobs paralleles game-portal :
//!   - health-check    (30s)
//!   - idle-shutdown   (1h)
//!   - reconcile       (1h)
//!   - image-cleanup   (24h)
//!   - reveal-ip       (5min)
//!   - daily-ping      (1h)
//!
//! Chacun POST sur /api/games/internal/jobs/{job} et logge le resultat.
//! Porte de sentinel-worker/src/domains/game_portal/jobs.rs (auth Bearer via
//! NEXUS_API_KEY au lieu de API_KEY).

use std::time::Duration;

use serde::Deserialize;
use tracing::{error, info};

const WORKER_NAME: &str = "game-portal";

#[derive(Debug, Deserialize, Default)]
struct JobReport {
    #[serde(default)]
    processed: usize,
    #[serde(default)]
    errors: usize,
    #[serde(default)]
    details: serde_json::Value,
}

/// Intervalles (secondes) des 6 jobs periodiques game-portal, resolus depuis
/// l'environnement (voir `main.rs`).
pub struct GamePortalIntervals {
    pub health_check_secs: u64,
    pub idle_shutdown_secs: u64,
    pub reconciler_secs: u64,
    pub image_cleanup_secs: u64,
    pub reveal_ip_secs: u64,
    pub daily_ping_secs: u64,
}

/// Spawn les 6 tasks paralleles. Ne bloque pas l'appelant.
pub fn start(api_url: String, intervals: GamePortalIntervals) {
    let api_key = std::env::var("NEXUS_API_KEY").unwrap_or_default();

    // 60s etait trop court : health-check / idle-shutdown / reconcile font
    // un appel RCON sequentiel par serveur (jusqu'a rcon_timeout_secs chacun) ;
    // sur une flotte un peu grande le cumul depassait 60s -> job en echec +
    // execution partielle (les derniers serveurs jamais traites). MissedTick
    // Skip empeche le chevauchement, donc un timeout large est sur.
    let http = match reqwest::Client::builder()
        .timeout(Duration::from_secs(240))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "game_portal: HTTP client init failed");
            return;
        }
    };

    spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "health-check",
        Duration::from_secs(intervals.health_check_secs),
    );
    spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "idle-shutdown",
        Duration::from_secs(intervals.idle_shutdown_secs),
    );
    spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "reconcile",
        Duration::from_secs(intervals.reconciler_secs),
    );
    spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "image-cleanup",
        Duration::from_secs(intervals.image_cleanup_secs),
    );
    // Sessions : revelation d'IP a l'echeance + ping quotidien.
    spawn_job(
        http.clone(),
        api_url.clone(),
        api_key.clone(),
        "reveal-ip",
        Duration::from_secs(intervals.reveal_ip_secs),
    );
    spawn_job(
        http,
        api_url,
        api_key,
        "daily-ping",
        Duration::from_secs(intervals.daily_ping_secs),
    );
}

fn spawn_job(
    http: reqwest::Client,
    api_url: String,
    api_key: String,
    job: &'static str,
    interval: Duration,
) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(interval);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            match call_job(&http, &api_url, &api_key, job).await {
                Ok(report) => {
                    info!(
                        job = job,
                        processed = report.processed,
                        errors = report.errors,
                        "game_portal tick OK"
                    );
                    if report.errors > 0 {
                        platform_common_worker::send_worker_log(
                            &api_url,
                            WORKER_NAME,
                            "warn",
                            job,
                            &format!("job {} : {} erreurs", job, report.errors),
                            serde_json::json!({
                                "event_type": format!("game_portal.{}.errors", job),
                                "report": report.details,
                            }),
                        )
                        .await;
                    }
                }
                Err(e) => {
                    error!(error = %e, job, "game_portal tick failed");
                    platform_common_worker::send_worker_log(
                        &api_url,
                        WORKER_NAME,
                        "error",
                        job,
                        &format!("job {} echec: {}", job, e),
                        serde_json::json!({
                            "event_type": format!("game_portal.{}.error", job),
                            "error": e,
                        }),
                    )
                    .await;
                }
            }
        }
    });
}

async fn call_job(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    job: &str,
) -> Result<JobReport, String> {
    let url = format!("{api_url}/api/games/internal/jobs/{job}");
    let mut req = http.post(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().await.map_err(|e| format!("HTTP send: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HTTP {status}: {body}"));
    }
    resp.json::<JobReport>()
        .await
        .map_err(|e| format!("decode JobReport: {e}"))
}
