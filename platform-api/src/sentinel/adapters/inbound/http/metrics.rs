//! Métriques Prometheus de l'API.
//!
//! Architecture :
//! - `init_prometheus()` installe un recorder Prometheus global. À appeler
//!   **une seule fois** au démarrage (avant `Router::build`).
//! - `metrics_handler()` expose `/metrics` au format texte Prometheus.
//! - `metrics_middleware()` instrumente chaque requête HTTP avec un counter
//!   et un histogramme de latence par `(route_pattern, method, status)`.
//! - `spawn_tokio_runtime_sampler()` échantillonne les métriques runtime tokio
//!   toutes les 10s (busy_ratio, queue_depth, ...).
//!
//! ⚠️ **Cardinality** : on utilise le `MatchedPath` Axum (le pattern de route,
//! pas l'URI réelle) pour éviter une explosion de séries — `/users/123` et
//! `/users/456` partagent le label `/users/{id}`.

use axum::extract::MatchedPath;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;
/// Handle global vers le recorder Prometheus.
///
/// Initialisé une seule fois via `init_prometheus()` au démarrage.
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Buckets pour l'histogramme de latence HTTP (en secondes).
///
/// Granularité fine sur 1ms-1s (cas normal API web), plus 2 buckets larges
/// pour capturer les outliers (analyses IA, batch).
const HTTP_LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// Installe le recorder Prometheus global.
///
/// Doit être appelée **avant** d'enregistrer la moindre métrique. Idempotente :
/// les appels suivants sont silencieusement ignorés.
pub fn init_prometheus() {
    if PROMETHEUS_HANDLE.get().is_some() {
        return;
    }

    let handle = PrometheusBuilder::new()
        .set_buckets_for_metric(
            metrics_exporter_prometheus::Matcher::Full("http_request_duration_seconds".to_string()),
            HTTP_LATENCY_BUCKETS,
        )
        .expect("buckets HTTP latency valides")
        .install_recorder()
        .expect("Prometheus recorder installé une seule fois");

    let _ = PROMETHEUS_HANDLE.set(handle);
}

/// Handler Axum exposant `/metrics` au format texte Prometheus.
///
/// Si `init_prometheus()` n'a pas été appelée, retourne une chaîne vide
/// (Prometheus considère ça comme "pas de métrique" et ne lève pas d'erreur).
pub async fn metrics_handler(
    axum::extract::State(state): axum::extract::State<
        crate::sentinel::bootstrap::state::SharedState,
    >,
    headers: axum::http::HeaderMap,
) -> Response {
    // Protection optionnelle : si METRICS_TOKEN est defini, on exige
    // `Authorization: Bearer <token>`. Vide = ouvert (comportement historique :
    // Prometheus scrape sans auth sur le reseau interne). Mitige la fuite
    // d'infos operationnelles si le port venait a etre expose.
    let auth_header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());
    if !metrics_auth_ok(&state.metrics_token, auth_header) {
        return (axum::http::StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    render_metrics()
}

/// Decision d'auth pure (testable) : autorise si aucun token configure, sinon
/// exige un header `Authorization: Bearer <token>` egal en temps constant.
fn metrics_auth_ok(configured_token: &str, auth_header: Option<&str>) -> bool {
    if configured_token.is_empty() {
        return true;
    }
    use subtle::ConstantTimeEq;
    let provided = auth_header
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    provided
        .as_bytes()
        .ct_eq(configured_token.as_bytes())
        .into()
}

/// Rend les metriques Prometheus (sans controle d'acces).
fn render_metrics() -> Response {
    match PROMETHEUS_HANDLE.get() {
        Some(handle) => handle.render().into_response(),
        None => String::new().into_response(),
    }
}

/// Middleware Axum qui enregistre :
/// - `http_requests_total{route, method, status}` : counter
/// - `http_request_duration_seconds{route, method, status}` : histogram
///
/// Le `route` est le pattern matché (ex : `/api/levels/{guild_id}/users/{user_id}`),
/// pas l'URI réelle, pour borner la cardinalité.
pub async fn metrics_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let start = Instant::now();

    // Extraire le pattern de route si Axum l'a déjà matché (via Router).
    // Sinon, fallback "unknown" pour ne pas créer une série par URI brute.
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let method = req.method().as_str().to_string();

    let response = next.run(req).await;

    let status = response.status().as_u16().to_string();
    let elapsed = start.elapsed().as_secs_f64();

    metrics::counter!(
        "http_requests_total",
        "route" => route.clone(),
        "method" => method.clone(),
        "status" => status.clone(),
    )
    .increment(1);

    metrics::histogram!(
        "http_request_duration_seconds",
        "route" => route,
        "method" => method,
        "status" => status,
    )
    .record(elapsed);

    response
}

/// Démarre une boucle qui échantillonne les métriques runtime tokio toutes les
/// 10 secondes (configurable via `TOKIO_METRICS_INTERVAL_SECS`).
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
/// après `init_prometheus`).
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::Method;
    use axum::http::Request;
    use axum::http::StatusCode;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    // Serialise l'init Prometheus entre les tests paralleles — le recorder
    // global metrics-rs ne peut etre installe qu'une seule fois par process.
    static TEST_INIT: std::sync::Once = std::sync::Once::new();
    fn ensure_init() {
        TEST_INIT.call_once(init_prometheus);
    }

    #[tokio::test]
    async fn metrics_handler_ok_status() {
        ensure_init();
        let resp = render_metrics();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[test]
    fn metrics_auth_open_when_no_token() {
        // Aucun token configure -> ouvert (comportement historique).
        assert!(metrics_auth_ok("", None));
        assert!(metrics_auth_ok("", Some("Bearer whatever")));
    }

    #[test]
    fn metrics_auth_requires_matching_bearer() {
        assert!(metrics_auth_ok("s3cret", Some("Bearer s3cret")));
        // Mauvais token, pas de header, mauvais schema -> refuse.
        assert!(!metrics_auth_ok("s3cret", Some("Bearer nope")));
        assert!(!metrics_auth_ok("s3cret", None));
        assert!(!metrics_auth_ok("s3cret", Some("s3cret"))); // sans prefixe Bearer
    }

    #[tokio::test]
    async fn init_prometheus_is_idempotent() {
        ensure_init();
        // Le 2e appel via le guard OnceLock doit sortir immediatement (return precoce).
        init_prometheus();
        assert!(PROMETHEUS_HANDLE.get().is_some());
    }

    #[tokio::test]
    async fn metrics_handler_renders_after_init() {
        ensure_init();
        let resp = render_metrics();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
        // Prometheus render est du texte (peut etre vide si pas encore de metriques,
        // mais apres l'avoir initialise + middleware, on devrait avoir du contenu).
        let _ = std::str::from_utf8(&body).unwrap();
    }

    async fn dummy_ok() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn middleware_records_request_without_matched_path() {
        ensure_init();
        let app = Router::new()
            .route("/ping", get(dummy_ok))
            .layer(from_fn(metrics_middleware));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/ping")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        // Verifier qu'au moins une metrique http_requests_total a ete enregistree.
        let render = match PROMETHEUS_HANDLE.get() {
            Some(h) => h.render(),
            None => String::new(),
        };
        assert!(render.contains("http_requests_total"));
    }

    #[tokio::test]
    async fn middleware_records_request_with_different_status() {
        ensure_init();
        async fn not_found() -> (StatusCode, &'static str) {
            (StatusCode::NOT_FOUND, "nope")
        }
        let app = Router::new()
            .route("/missing", get(not_found))
            .layer(from_fn(metrics_middleware));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/missing")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
