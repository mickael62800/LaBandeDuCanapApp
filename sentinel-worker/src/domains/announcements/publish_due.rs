//! Publie les annonces dues sur la stream Redis a chaque heure pile.
//! Porte de l'ancien announcement-worker/main.rs (logique inchangee).

use std::time::Duration;

use chrono::{Timelike, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tracing::{error, info, warn};

use platform_common_worker::{self, JobMetrics, SupervisedTask};

const WORKER_NAME: &str = "announcements";
const STREAM_KEY: &str = "sentinel:events";
const STREAM_MAXLEN: usize = 10_000;
/// Plafond global passe a `/api/announcements/internal/due`. L'API
/// applique ensuite le cap par-guild via la cle config `fetch_limit`.
/// Override possible via env `ANNOUNCEMENTS_FETCH_LIMIT_GLOBAL` si on
/// veut throttle l'ensemble du tick.
const DEFAULT_FETCH_LIMIT_GLOBAL: i64 = 200;

fn fetch_limit_global() -> i64 {
    std::env::var("ANNOUNCEMENTS_FETCH_LIMIT_GLOBAL")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_FETCH_LIMIT_GLOBAL)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedEmbed {
    title: Option<String>,
    description: String,
    color: Option<i32>,
    image_url: Option<String>,
    thumbnail_url: Option<String>,
    #[serde(default)]
    footer_text: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct RenderedAnnouncement {
    announcement_id: String,
    run_id: String,
    guild_id: String,
    channel_ids: Vec<String>,
    content_text: String,
    embed: Option<RenderedEmbed>,
    mentions_prefix: String,
    // Champs transportes tels quels vers le bot (le worker ne fait que relayer).
    #[serde(default)]
    buttons: serde_json::Value,
    #[serde(default)]
    auto_reactions: serde_json::Value,
}

/// Spawn la boucle d'annonces : aligne sur HH:00:00 UTC, puis tick
/// toutes les heures. Ne bloque pas l'appelant.
pub fn start(
    http_client: reqwest::Client,
    api_url: String,
    mut redis: redis::aio::ConnectionManager,
    publish_interval_secs: u64,
    mut shutdown: watch::Receiver<bool>,
) -> SupervisedTask {
    SupervisedTask::spawn("publish_due_announcements", async move {
        let api_key = std::env::var("API_KEY").unwrap_or_default();
        let mut job_metrics = JobMetrics::new("publish_due_announcements", WORKER_NAME);

        let initial_delay = compute_initial_delay();
        info!(
            delay_secs = initial_delay.as_secs(),
            "announcements: aligning on next hour boundary"
        );
        tokio::select! {
            biased;
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    job_metrics.stopped();
                    return;
                }
            }
            _ = tokio::time::sleep(initial_delay) => {}
        }

        let mut interval = tokio::time::interval(Duration::from_secs(publish_interval_secs.max(1)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
                _ = interval.tick() => {
                    let started = std::time::Instant::now();
                    job_metrics.started();
                    if let Err(e) = run_one_tick(&http_client, &api_url, &api_key, &mut redis).await {
                        job_metrics.failed(started.elapsed());
                        error!(error = %e, "announcements tick error");
                        platform_common_worker::send_worker_log(
                            &api_url,
                            WORKER_NAME,
                            "error",
                            "tick",
                            &format!("Tick error: {e}"),
                            serde_json::json!({ "event_type": "announcement.tick.error", "error": e }),
                        )
                        .await;
                    } else {
                        job_metrics.succeeded(started.elapsed());
                    }
                }
            }
        }

        job_metrics.stopped();
        info!("announcements: boucle arretee (shutdown)");
    })
}

fn compute_initial_delay() -> Duration {
    let now = Utc::now();
    Duration::from_secs(
        sentinel_core::domain::services::system::scheduling::secs_to_next_hour(
            now.minute(),
            now.second(),
        ),
    )
}

async fn run_one_tick(
    http: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    redis: &mut redis::aio::ConnectionManager,
) -> Result<(), String> {
    let url = format!(
        "{api_url}/api/announcements/internal/due?limit={}",
        fetch_limit_global()
    );
    let mut req = http.get(&url);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("HTTP fetch_due: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("fetch_due returned {status}: {body}"));
    }
    let payloads: Vec<RenderedAnnouncement> = resp
        .json()
        .await
        .map_err(|e| format!("Decode fetch_due response: {e}"))?;

    if payloads.is_empty() {
        info!("announcements: no announcements due, skip tick");
        return Ok(());
    }

    info!(
        count = payloads.len(),
        "announcements: publishing via Redis stream"
    );

    for p in &payloads {
        let payload_json = serde_json::to_string(&serde_json::json!({
            "event": "announcement_publish",
            "data": p,
        }))
        .map_err(|e| format!("encode payload: {e}"))?;

        let res: redis::RedisResult<String> = redis
            .xadd_maxlen(
                STREAM_KEY,
                redis::streams::StreamMaxlen::Approx(STREAM_MAXLEN),
                "*",
                &[("payload", payload_json.as_str())],
            )
            .await;
        match res {
            Ok(id) => {
                info!(stream_id = %id, run_id = %p.run_id, channels = p.channel_ids.len(), "XADD success");
            }
            Err(e) => {
                warn!(error = %e, run_id = %p.run_id, "XADD failed");
                platform_common_worker::send_worker_log(
                    api_url,
                    WORKER_NAME,
                    "warn",
                    "tick",
                    "XADD failed",
                    serde_json::json!({
                        "event_type": "announcement.xadd.error",
                        "run_id": p.run_id,
                        "error": e.to_string(),
                    }),
                )
                .await;
            }
        }
    }

    platform_common_worker::send_worker_log(
        api_url,
        WORKER_NAME,
        "info",
        "tick",
        &format!("Published {} announcement(s)", payloads.len()),
        serde_json::json!({
            "event_type": "announcement.tick.success",
            "count": payloads.len(),
        }),
    )
    .await;

    Ok(())
}
