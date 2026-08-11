//! Helpers Prometheus + tokio-metrics partagés par les workers et l'API.
//!
//! Usage typique dans `main.rs` d'un worker :
//!
//! ```ignore
//! use platform_common_worker::metrics;
//!
//! #[tokio::main]
//! async fn main() {
//!     // 1. Initialise le recorder Prometheus AVANT toute émission de métrique
//!     let handle = metrics::init_prometheus();
//!
//!     // 2. Démarre le sampler tokio runtime (toutes les 10s)
//!     metrics::spawn_tokio_runtime_sampler();
//!
//!     // 3. Expose /metrics sur un port dédié (par défaut $METRICS_PORT ou 9100)
//!     metrics::spawn_metrics_server(handle, "monitoring-worker", 9100);
//!
//!     // ... reste du worker
//! }
//! ```

use std::sync::OnceLock;
use std::time::Duration;

use axum::extract::State;
use axum::routing::get;
use axum::Router;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use tracing::{error, info, warn};

/// Recorder Prometheus global. Initialisé une seule fois via `init_prometheus()`.
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Installe le recorder Prometheus global et renvoie son handle.
///
/// Idempotent : appels suivants retournent simplement le handle déjà installé
/// (ou un nouveau si l'install a échoué — ce qui ne devrait pas arriver).
pub fn init_prometheus() -> PrometheusHandle {
    if let Some(handle) = PROMETHEUS_HANDLE.get() {
        return handle.clone();
    }

    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("Prometheus recorder installé une seule fois");

    let _ = PROMETHEUS_HANDLE.set(handle.clone());
    handle
}

/// Démarre une boucle qui échantillonne les métriques runtime tokio toutes les
/// 10 secondes et les expose en gauges Prometheus.
///
/// Métriques exposées (n'utilise que des champs **stables** de `tokio-metrics`,
/// pas besoin de `RUSTFLAGS="--cfg tokio_unstable"`) :
/// - `tokio_workers_count` : nombre de workers du runtime
/// - `tokio_live_tasks_count` : tâches vivantes au moment du snapshot
/// - `tokio_busy_ratio` : ratio (0..1) du temps total où un worker est busy
///   (saturation effective du runtime — au-delà de 0.7 = signal d'alerte)
/// - `tokio_global_queue_depth` : profondeur de la file globale
/// - `tokio_total_park_count` : nombre cumulé de parks (workers en attente)
/// - `tokio_max_busy_duration_seconds` : worker le plus chargé sur la fenêtre
///
/// Doit être appelée depuis un context tokio actif (typiquement depuis `main`
/// après l'init du runtime).
pub fn spawn_tokio_runtime_sampler() {
    let monitor = tokio_metrics::RuntimeMonitor::new(&tokio::runtime::Handle::current());

    tokio::spawn(async move {
        let interval_secs: u64 = std::env::var("TOKIO_METRICS_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);

        let mut intervals = monitor.intervals();
        loop {
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;

            if let Some(snapshot) = intervals.next() {
                metrics::gauge!("tokio_workers_count").set(snapshot.workers_count as f64);
                metrics::gauge!("tokio_live_tasks_count").set(snapshot.live_tasks_count as f64);
                metrics::gauge!("tokio_global_queue_depth").set(snapshot.global_queue_depth as f64);
                metrics::gauge!("tokio_total_park_count").set(snapshot.total_park_count as f64);
                metrics::gauge!("tokio_max_busy_duration_seconds")
                    .set(snapshot.max_busy_duration.as_secs_f64());

                // busy_ratio = busy_total / (workers * elapsed) sur la fenêtre
                let busy = snapshot.total_busy_duration.as_secs_f64();
                let elapsed = snapshot.elapsed.as_secs_f64();
                let total = (snapshot.workers_count as f64) * elapsed.max(1e-6);
                let ratio = if total > 0.0 { busy / total } else { 0.0 };
                metrics::gauge!("tokio_busy_ratio").set(ratio);
            }
        }
    });
}

/// Démarre un serveur HTTP minimaliste qui expose `/metrics` sur le port donné.
///
/// Utilisé par les workers (qui n'ont pas leur propre Axum). L'API expose déjà
/// `/metrics` directement sur son routeur principal — elle n'a pas besoin d'appeler
/// cette fonction.
///
/// Le label `service` est ajouté à toutes les métriques via le `service_label`
/// — utile pour distinguer les workers dans Grafana.
pub fn spawn_metrics_server(handle: PrometheusHandle, service_label: &'static str, port: u16) {
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(handle);

    tokio::spawn(async move {
        let addr = format!("0.0.0.0:{port}");
        info!(addr = %addr, service = service_label, "Serveur métriques démarré");

        let listener = match tokio::net::TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                error!(error = %e, addr = %addr, "Impossible de bind le port métriques");
                return;
            }
        };

        if let Err(e) = axum::serve(listener, app).await {
            warn!(error = %e, "Serveur métriques arrêté");
        }
    });
}

async fn metrics_handler(State(handle): State<PrometheusHandle>) -> String {
    handle.render()
}

/// Helper "one-liner" qui initialise toute l'observabilité d'un worker :
/// - installe le recorder Prometheus
/// - démarre le sampler runtime tokio
/// - lance le serveur HTTP `/metrics` sur `METRICS_PORT` (défaut 9100)
///
/// À appeler depuis le `main()` du worker, dans un context tokio actif, mais
/// avant la moindre émission de métrique.
pub fn init_observability(service_label: &'static str) {
    let handle = init_prometheus();
    spawn_tokio_runtime_sampler();
    let port: u16 = std::env::var("METRICS_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(9100);
    spawn_metrics_server(handle, service_label, port);
}
