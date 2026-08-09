//! Publie les annonces dues sur la stream Redis a chaque heure pile.
//! Porte de l'ancien announcement-worker/main.rs (logique inchangee).

use std::time::Duration;

use chrono::{Timelike, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use platform_common_worker;

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
pub fn start(api_url: String, redis_client: redis::Client, publish_interval_secs: u64) {
    tokio::spawn(async move {
        let api_key = std::env::var("API_KEY").unwrap_or_default();

        let initial_delay = compute_initial_delay();
        info!(
            delay_secs = initial_delay.as_secs(),
            "announcements: aligning on next hour boundary"
        );
        tokio::time::sleep(initial_delay).await;

        let http_client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "announcements: HTTP client init failed");
                return;
            }
        };

        let mut interval = tokio::time::interval(Duration::from_secs(publish_interval_secs));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            interval.tick().await;
            if let Err(e) = run_one_tick(&http_client, &api_url, &api_key, &redis_client).await {
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
            }
        }
    });
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
    redis_client: &redis::Client,
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

    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| format!("Redis conn: {e}"))?;

    for p in &payloads {
        let payload_json = serde_json::to_string(&serde_json::json!({
            "event": "announcement_publish",
            "data": p,
        }))
        .map_err(|e| format!("encode payload: {e}"))?;

        let res: redis::RedisResult<String> = conn
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

