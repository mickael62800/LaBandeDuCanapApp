use std::future::Future;
use std::time::Duration;

pub fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub fn spawn_interval<F, Fut>(name: &'static str, interval_secs: u64, mut job: F)
where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    tracing::info!(job = name, interval_secs, "job planifie");
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs.max(1)));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            let started = std::time::Instant::now();
            match job().await {
                Ok(()) => {
                    metrics::counter!("scheduler_job_runs_total", "job" => name, "status" => "success").increment(1);
                    metrics::gauge!("scheduler_job_last_success_timestamp_seconds", "job" => name).set(unix_now());
                    metrics::histogram!("scheduler_job_duration_seconds", "job" => name).record(started.elapsed().as_secs_f64());
                    tracing::info!(
                    job = name,
                    elapsed_ms = started.elapsed().as_millis(),
                    "job termine"
                    )
                },
                Err(error) => {
                    metrics::counter!("scheduler_job_runs_total", "job" => name, "status" => "error").increment(1);
                    metrics::gauge!("scheduler_job_last_error_timestamp_seconds", "job" => name).set(unix_now());
                    tracing::error!(job = name, %error, "job en echec")
                },
            }
        }
    });
}

fn unix_now() -> f64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs_f64() }
