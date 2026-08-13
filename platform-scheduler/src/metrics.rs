use std::sync::OnceLock;

use axum::{extract::State, routing::get, Router};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn init() {
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("recorder Prometheus");
    let _ = HANDLE.set(handle.clone());
    tokio::spawn(async move {
        let port = std::env::var("METRICS_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(9100);
        let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
            .await
            .expect("bind metrics");
        let app = Router::new()
            .route("/metrics", get(render))
            .with_state(handle);
        if let Err(error) = axum::serve(listener, app).await {
            tracing::error!(%error, "serveur metrics arrete");
        }
    });
}

async fn render(State(handle): State<PrometheusHandle>) -> String {
    handle.render()
}
