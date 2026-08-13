use platform_api::nexus::{adapters, bootstrap};
use tokio::signal;

#[tokio::main]
async fn main() {
    run().await;
}

pub async fn run() {
    dotenvy::dotenv().ok();
    if std::env::var_os("PLATFORM_API_UNIFIED_RUNTIME").is_none() {
        tracing_subscriber::fmt::init();
    }

    adapters::inbound::http::metrics::init_prometheus();
    adapters::inbound::http::metrics::spawn_tokio_runtime_sampler();

    let state = match bootstrap::build_state().await {
        Ok(state) => state,
        Err(error) => {
            tracing::error!(%error, "bootstrap nexus-api impossible");
            std::process::exit(1);
        }
    };
    let port = std::env::var("NEXUS_API_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(3100);
    let app = adapters::inbound::http::build_router(state);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind nexus-api");
    tracing::info!(%addr, "nexus-api demarree depuis platform-api");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("serve nexus-api");
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("ecoute Ctrl+C") };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("ecoute SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Ctrl+C recu"),
        _ = terminate => tracing::info!("SIGTERM recu"),
    }
}
