//! Jobs periodiques du portail de jeux, delegues a `nexus-api`.

use std::time::Duration;

use platform_common_worker::http_job::HttpJobClient;
use serde::Deserialize;
use tracing::info;

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

pub struct GamePortalIntervals {
    pub health_check_secs: u64,
    pub idle_shutdown_secs: u64,
    pub reconciler_secs: u64,
    pub image_cleanup_secs: u64,
    pub reveal_ip_secs: u64,
    pub daily_ping_secs: u64,
}

pub fn start(client: HttpJobClient, api_url: String, intervals: GamePortalIntervals) {
    for (job, interval_secs) in [
        ("health-check", intervals.health_check_secs),
        ("idle-shutdown", intervals.idle_shutdown_secs),
        ("reconcile", intervals.reconciler_secs),
        ("image-cleanup", intervals.image_cleanup_secs),
        ("reveal-ip", intervals.reveal_ip_secs),
        ("daily-ping", intervals.daily_ping_secs),
    ] {
        spawn_job(client.clone(), api_url.clone(), job, interval_secs);
    }
}

fn spawn_job(client: HttpJobClient, api_url: String, job: &'static str, interval_secs: u64) {
    platform_common_worker::spawn_interval(job, interval_secs, move || {
        let client = client.clone();
        let api_url = api_url.clone();
        async move {
            let path = format!("/api/games/internal/jobs/{job}");
            match client
                .post_json_with_timeout::<JobReport>(&path, Duration::from_secs(240))
                .await
            {
                Ok(report) => {
                    info!(
                        job,
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
                            &format!("job {job} : {} erreurs", report.errors),
                            serde_json::json!({
                                "event_type": format!("game_portal.{job}.errors"),
                                "report": report.details,
                            }),
                        )
                        .await;
                    }
                    Ok(())
                }
                Err(error) => {
                    platform_common_worker::send_worker_log(
                        &api_url,
                        WORKER_NAME,
                        "error",
                        job,
                        &format!("job {job} echec: {error}"),
                        serde_json::json!({
                            "event_type": format!("game_portal.{job}.error"),
                            "error": error,
                        }),
                    )
                    .await;
                    Err(error)
                }
            }
        }
    });
}
