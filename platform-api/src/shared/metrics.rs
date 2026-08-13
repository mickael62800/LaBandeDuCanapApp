//! Metriques Prometheus partagees par les deux APIs.
//!
//! Les deux exposent les **memes noms de metriques** (`http_requests_total`,
//! `http_request_duration_seconds`, `tokio_*`), ce qui permet de reutiliser les
//! memes dashboards Grafana en filtrant sur le label `service`.
//!
//! Ce module ne fournit PAS le handler `/metrics` : celui-ci depend de l'etat
//! applicatif (pour lire le jeton), donc chaque API ecrit le sien en trois
//! lignes au-dessus de [`metrics_auth_ok`] et [`render_metrics`].

use std::sync::OnceLock;
use std::time::Duration;
use std::time::Instant;

use axum::extract::MatchedPath;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum::response::Response;
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_exporter_prometheus::PrometheusHandle;

/// Handle global vers le recorder. Installe une seule fois par process.
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Buckets de latence HTTP par defaut, en secondes.
///
/// Granularite fine sous la seconde, et jusqu'a 30 s pour les operations
/// longues (provisioning de conteneur, inference lourde) : les voir toutes
/// s'entasser dans le dernier bucket ne dirait rien.
pub const DEFAULT_LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// Installe le recorder Prometheus global.
///
/// A appeler AVANT la construction du routeur : une metrique emise avant
/// l'installation du recorder est perdue. Idempotente.
pub fn init_prometheus() {
    init_prometheus_with_buckets(DEFAULT_LATENCY_BUCKETS);
}

/// Variante avec des buckets de latence explicites.
pub fn init_prometheus_with_buckets(buckets: &[f64]) {
    // `get()` suivi de `set()` n'est pas atomique : deux domaines demarrant en
    // parallele pouvaient tous deux tenter d'installer le recorder global.
    // `get_or_init` garantit qu'une seule construction a lieu par processus.
    PROMETHEUS_HANDLE.get_or_init(|| {
        PrometheusBuilder::new()
            .set_buckets_for_metric(
                metrics_exporter_prometheus::Matcher::Full(
                    "http_request_duration_seconds".to_string(),
                ),
                buckets,
            )
            .expect("buckets HTTP latency valides")
            .install_recorder()
            .expect("recorder Prometheus installe une seule fois")
    });
}

/// Rend les metriques au format texte Prometheus, sans controle d'acces.
///
/// Si le recorder n'est pas installe, rend une page vide : Prometheus lit ca
/// comme « aucune metrique », pas comme une panne.
pub fn render_metrics() -> Response {
    match PROMETHEUS_HANDLE.get() {
        Some(handle) => handle.render().into_response(),
        None => String::new().into_response(),
    }
}

/// Decision d'auth pour `/metrics`, pure et donc testable sans routeur.
///
/// Un jeton vide ou absent = endpoint ouvert, ce qui convient tant que le port
/// n'est joignable que depuis le reseau interne ou vit Prometheus.
///
/// Comparaison constant-time : une comparaison naive s'arrete au premier octet
/// different, ce qui laisse deviner le jeton caractere par caractere en
/// mesurant le temps de reponse.
pub fn metrics_auth_ok(configured: Option<&str>, auth_header: Option<&str>) -> bool {
    let Some(expected) = configured.filter(|t| !t.is_empty()) else {
        return true;
    };
    use subtle::ConstantTimeEq;
    let provided = auth_header
        .and_then(|s| s.strip_prefix("Bearer "))
        .unwrap_or("");
    provided.as_bytes().ct_eq(expected.as_bytes()).into()
}

/// Middleware qui enregistre, par `(route, method, status)` :
/// - `http_requests_total` (counter)
/// - `http_request_duration_seconds` (histogram)
///
/// Doit etre pose APRES le matching de route, sinon `MatchedPath` est absent et
/// tout retombe sur le label `unknown`.
///
/// Cardinalite : on etiquette avec le motif de route, jamais l'URI reelle.
/// Sans ca, `/api/wallet/123/456` creerait une serie par joueur.
pub async fn metrics_middleware(req: Request<axum::body::Body>, next: Next) -> Response {
    let start = Instant::now();

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

/// Echantillonne le runtime tokio vers des gauges, toutes les N secondes.
///
/// N'utilise que des champs stables de `tokio-metrics` : pas besoin de
/// `RUSTFLAGS="--cfg tokio_unstable"`. A appeler depuis un contexte tokio.
///
/// `tokio_busy_ratio` est la metrique a surveiller : au-dela de 0.7, le runtime
/// sature — typiquement une operation bloquante qui ne rend pas la main.
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
    use axum::body::Body;
    use axum::http::Method;
    use axum::http::StatusCode;
    use axum::middleware::from_fn;
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    // Le recorder global ne s'installe qu'une fois par process : les tests
    // paralleles doivent passer par ce garde.
    static TEST_INIT: std::sync::Once = std::sync::Once::new();
    fn ensure_init() {
        TEST_INIT.call_once(init_prometheus);
    }

    #[test]
    fn metrics_ouvert_sans_token() {
        assert!(metrics_auth_ok(None, None));
        assert!(metrics_auth_ok(Some(""), None));
        assert!(metrics_auth_ok(None, Some("Bearer peu importe")));
    }

    #[test]
    fn metrics_exige_le_bon_bearer() {
        assert!(metrics_auth_ok(Some("s3cret"), Some("Bearer s3cret")));
        assert!(!metrics_auth_ok(Some("s3cret"), Some("Bearer nope")));
        assert!(!metrics_auth_ok(Some("s3cret"), None));
        // Sans le prefixe Bearer : refuse.
        assert!(!metrics_auth_ok(Some("s3cret"), Some("s3cret")));
        // Un token plus court ne doit pas passer par troncature.
        assert!(!metrics_auth_ok(Some("s3cret"), Some("Bearer s3c")));
    }

    #[tokio::test]
    async fn init_est_idempotent() {
        ensure_init();
        init_prometheus();
        assert!(PROMETHEUS_HANDLE.get().is_some());
    }

    #[tokio::test]
    async fn middleware_enregistre_la_requete() {
        ensure_init();
        async fn ok() -> &'static str {
            "ok"
        }
        let app = Router::new()
            .route("/ping", get(ok))
            .layer(from_fn(metrics_middleware));

        let req = Request::builder()
            .method(Method::GET)
            .uri("/ping")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let render = PROMETHEUS_HANDLE
            .get()
            .map(|h| h.render())
            .unwrap_or_default();
        assert!(render.contains("http_requests_total"));
    }
}
