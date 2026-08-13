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
use std::time::Instant;

/// Installe le recorder Prometheus global.
///
/// Doit être appelée **avant** d'enregistrer la moindre métrique. Idempotente :
/// les appels suivants sont silencieusement ignorés.
pub fn init_prometheus() {
    crate::shared::metrics::init_prometheus();
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
    crate::shared::metrics::render_metrics()
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
    crate::shared::metrics::spawn_tokio_runtime_sampler();
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
        let response = render_metrics();
        assert_eq!(response.status(), StatusCode::OK);
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
        let response = render_metrics();
        let body = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        let render = std::str::from_utf8(&body).unwrap();
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
