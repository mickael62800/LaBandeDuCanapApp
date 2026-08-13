use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub fn init_tracing(default_filter: &str) {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .init();
}

pub async fn create_pg_pool(database_url: &str) -> sqlx::PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .unwrap_or_else(|error| panic!("connexion PostgreSQL impossible: {error}"))
}

pub fn open_redis(redis_url: &str) -> redis::Client {
    redis::Client::open(redis_url).unwrap_or_else(|error| panic!("URL Redis invalide: {error}"))
}

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("signal SIGTERM");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

pub struct JobMetrics {
    job: &'static str,
    worker: &'static str,
    consecutive_errors: u64,
}

impl JobMetrics {
    pub fn new(job: &'static str, worker: &'static str) -> Self {
        metrics::gauge!("worker_job_alive", "job" => job, "worker" => worker).set(1.0);
        Self {
            job,
            worker,
            consecutive_errors: 0,
        }
    }
    pub fn started(&self) {
        metrics::gauge!("worker_job_last_start_timestamp_seconds", "job" => self.job, "worker" => self.worker).set(now());
    }
    pub fn succeeded(&mut self, duration: Duration) {
        self.consecutive_errors = 0;
        self.record(duration);
        metrics::gauge!("worker_job_last_success_timestamp_seconds", "job" => self.job, "worker" => self.worker).set(now());
    }
    pub fn failed(&mut self, duration: Duration) {
        self.consecutive_errors = self.consecutive_errors.saturating_add(1);
        self.record(duration);
        metrics::counter!("worker_job_errors_total", "job" => self.job, "worker" => self.worker)
            .increment(1);
    }
    pub fn stopped(&self) {
        metrics::gauge!("worker_job_alive", "job" => self.job, "worker" => self.worker).set(0.0);
    }
    fn record(&self, duration: Duration) {
        metrics::gauge!("worker_job_last_duration_seconds", "job" => self.job, "worker" => self.worker).set(duration.as_secs_f64());
        metrics::gauge!("worker_job_consecutive_errors", "job" => self.job, "worker" => self.worker).set(self.consecutive_errors as f64);
    }
}
fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
