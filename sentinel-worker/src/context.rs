//! Dependances partagees par tous les jobs du Worker.
//!
//! Le contexte est construit une seule fois apres resolution de la config DB.
//! Ses clones partagent le pool, le gestionnaire Redis auto-reconnectant et les
//! pools de connexions HTTP ; cloner le contexte ne recree aucune connexion.

use std::sync::Arc;
use std::time::Duration;

use redis::aio::ConnectionManager;
use sqlx::PgPool;
use tokio::sync::watch;

use crate::config::WorkerConfig;

const HTTP_CONNECT_TIMEOUT_SECS: u64 = 5;
const HTTP_STANDARD_TIMEOUT_SECS: u64 = 15;
const HTTP_LONG_TIMEOUT_SECS: u64 = 60;

#[derive(Clone)]
pub struct HttpClients {
    pub standard: reqwest::Client,
    pub long_running: reqwest::Client,
}

#[derive(Clone)]
pub struct WorkerContext {
    pub pool: PgPool,
    pub redis: ConnectionManager,
    pub http: HttpClients,
    pub config: Arc<WorkerConfig>,
    pub shutdown: watch::Receiver<bool>,
}

impl WorkerContext {
    pub async fn new(
        config: WorkerConfig,
        pool: PgPool,
        redis_client: redis::Client,
        shutdown: watch::Receiver<bool>,
    ) -> Result<Self, String> {
        let redis = ConnectionManager::new(redis_client)
            .await
            .map_err(|error| format!("redis connection manager: {error}"))?;
        let http = HttpClients {
            standard: build_http_client(HTTP_STANDARD_TIMEOUT_SECS)?,
            long_running: build_http_client(HTTP_LONG_TIMEOUT_SECS)?,
        };

        Ok(Self {
            pool,
            redis,
            http,
            config: Arc::new(config),
            shutdown,
        })
    }
}

fn build_http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(HTTP_CONNECT_TIMEOUT_SECS))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|error| format!("http client ({timeout_secs}s): {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_http_clients_are_constructible() {
        assert!(build_http_client(HTTP_STANDARD_TIMEOUT_SECS).is_ok());
        assert!(build_http_client(HTTP_LONG_TIMEOUT_SECS).is_ok());
    }
}
