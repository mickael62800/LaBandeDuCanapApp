//! Worker d'exploitation de la machine hote.

use std::sync::Arc;

mod alerts_dispatcher;
mod container_monitor;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    platform_common_worker::init_tracing("ops_worker=info");
    platform_common_worker::metrics::init_observability("ops-worker");

    let database_url = required("OPS_DATABASE_URL");
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://redis:6379".into());
    let docker_agent_url =
        std::env::var("DOCKER_AGENT_URL").unwrap_or_else(|_| "http://docker-agent:8095".into());
    let docker_agent_token = required("DOCKER_AGENT_TOKEN");

    let pool = platform_common_worker::create_pg_pool(&database_url).await;
    let redis_client = platform_common_worker::redis_helpers::open_or_exit(&redis_url);

    let docker_host: Arc<dyn ops_core::ports::outbound::docker_host::DockerHost> =
        Arc::new(ops_adapters::http_docker_host::HttpDockerHost::new(
            docker_agent_url,
            docker_agent_token,
        ));
    let server_events: Arc<
        dyn ops_core::ports::outbound::server_event_repository::ServerEventRepository,
    > = Arc::new(ops_adapters::server_event_repository::PgServerEventRepository::new(
        pool.clone(),
    ));

    let monitor = container_monitor::spawn(docker_host, server_events, redis_client.clone());
    alerts_dispatcher::spawn(pool.clone(), redis_client, Some(monitor));

    tracing::info!("ops-worker demarre");
    platform_common_worker::shutdown_signal().await;
    pool.close().await;
    tracing::info!("ops-worker arrete");
}

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} est requis"))
}
