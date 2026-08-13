//! Agent Ops isole des surfaces HTTP publiques.
//!
//! Il est le seul producteur des metriques `/host/proc` et du snapshot de
//! surveillance Docker. Les consommateurs utilisent Redis; aucun etat memoire
//! n'est partage avec le scheduler ou les APIs.

use std::sync::Arc;

mod container_monitor;
mod host_metrics;
mod observability;
mod runtime;
mod service_monitor;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    runtime::init_tracing("ops_agent=info");
    observability::init_observability("ops-agent");

    let database_url = required("OPS_DATABASE_URL");
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".into());
    let docker_agent_url =
        std::env::var("DOCKER_AGENT_URL").unwrap_or_else(|_| "http://docker-agent:8095".into());
    let docker_agent_token = required("DOCKER_AGENT_TOKEN");
    let sentinel_api_url =
        std::env::var("SENTINEL_API_URL").unwrap_or_else(|_| "http://api:3000".into());
    let sentinel_api_key = std::env::var("SENTINEL_API_KEY").unwrap_or_default();
    let monitor_interval = std::env::var("MONITOR_CHECK_INTERVAL")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30);

    let pool = runtime::create_pg_pool(&database_url).await;
    let redis_client = runtime::open_redis(&redis_url);
    let docker_host: Arc<dyn platform_core::ops::ports::outbound::docker_host::DockerHost> =
        Arc::new(ops_adapters::http_docker_host::HttpDockerHost::new(
            docker_agent_url,
            docker_agent_token,
        ));
    let server_events: Arc<
        dyn platform_core::ops::ports::outbound::server_event_repository::ServerEventRepository,
    > = Arc::new(ops_adapters::server_event_repository::PgServerEventRepository::new(pool.clone()));

    container_monitor::spawn(docker_host, server_events, redis_client.clone());
    host_metrics::spawn(redis_client.clone());

    let redis = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("connexion Redis du monitoring des services impossible");
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let service_monitor = service_monitor::start(
        reqwest::Client::new(),
        redis,
        service_monitor::MonitorConfig {
            api_url: sentinel_api_url,
            api_key: sentinel_api_key,
            check_interval_secs: monitor_interval,
        },
        shutdown_rx,
    );

    tracing::info!("ops-agent demarre");
    runtime::shutdown_signal().await;
    let _ = shutdown_tx.send(true);
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), service_monitor).await;
    pool.close().await;
    tracing::info!("ops-agent arrete");
}

fn required(name: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| panic!("{name} est requis"))
}
