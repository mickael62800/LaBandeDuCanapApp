// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod broadcaster;
mod config;
mod handler;
mod health;
mod logger;
mod redis_subscriber;

use std::sync::Arc;

use axum::http::{header, HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn, Span};

use crate::broadcaster::EventBroadcaster;
use crate::config::Config;
use crate::handler::{ws_handler, GatewayState};
use crate::logger::GatewayLogger;

/// Attend Ctrl+C ou SIGTERM. Inline depuis l'ancien
/// `sentinel_worker_common::shutdown_signal()` (la lib partagee a ete
/// absorbee dans sentinel-worker).
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "Impossible d'ecouter Ctrl+C");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => tracing::error!(error = %e, "Impossible d'ecouter SIGTERM"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C recu"),
        _ = terminate => info!("Signal SIGTERM recu"),
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "sentinel_gateway=info,tower_http=debug".into()),
        )
        .init();

    let config = Config::from_env();

    info!(
        addr = %config.bind_addr(),
        redis = %config.redis_url,
        channel = %config.redis_channel,
        max_connections = config.max_connections,
        broadcast_capacity = config.broadcast_capacity,
        "Demarrage de Sentinel Gateway"
    );

    // La cle Sentinel ne sert plus dans l'URL WebSocket. Elle reste un mode
    // Bearer standard pour les rares clients internes et pour le logger.
    if config.api_key.is_empty() {
        warn!("SENTINEL_API_KEY non definie — acces WebSocket interne par Bearer desactive");
    }
    if config.auth_api_token.is_empty() {
        warn!("AUTH_API_TOKEN non defini — authentification WebSocket par session indisponible");
    }

    // Le logger reutilise la configuration deja chargee : il ne relit pas une
    // variable d'environnement potentiellement differente.
    let gw_logger = GatewayLogger::new(config.api_url.clone(), config.api_key.clone());

    // Broadcaster local (capacite configurable)
    let broadcaster = Arc::new(EventBroadcaster::new(
        config.broadcast_capacity,
        config.max_connections,
    ));

    // Lancer le subscriber Redis en background avec exponential backoff
    let redis_broadcaster = broadcaster.clone();
    let redis_url = config.redis_url.clone();
    let redis_channel = config.redis_channel.clone();
    let redis_logger = gw_logger.clone();
    let redis_base_delay = config.redis_reconnect_delay_secs;
    let redis_max_delay = config.redis_reconnect_max_delay_secs;
    tokio::spawn(async move {
        redis_subscriber::run_redis_subscriber(
            &redis_url,
            &redis_channel,
            redis_broadcaster,
            redis_logger,
            redis_base_delay,
            redis_max_delay,
        )
        .await;
    });

    // CORS
    let cors = build_cors(&config.allowed_origins, config.cors_max_age_secs);

    // Routes
    let ws_state = GatewayState {
        broadcaster: broadcaster.clone(),
        api_key: config.api_key.clone(),
        auth_api_url: config.auth_api_url.clone(),
        auth_api_token: config.auth_api_token.clone(),
        logger: gw_logger.clone(),
        http_client: reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("reqwest client"),
        // Meme source que la CORS, mais applique par le handler : un handshake
        // WebSocket ne passe pas par la CORS du navigateur (cf. `origin_authorized`).
        allowed_origins: parse_origins(&config.allowed_origins),
    };

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            let request_id = request
                .headers()
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("-");
            tracing::info_span!(
                "http_request",
                method = %request.method(),
                uri = %request.uri(),
                request_id = %request_id,
            )
        })
        .on_response(
            |response: &axum::http::Response<_>, latency: std::time::Duration, _span: &Span| {
                tracing::info!(
                    status = response.status().as_u16(),
                    latency_ms = latency.as_millis() as u64,
                    "response"
                );
            },
        );

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(ws_state)
        .route("/health", get(health::health))
        .with_state(broadcaster.clone())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(trace_layer)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Impossible de bind le port");

    gw_logger.info(
        "Gateway WebSocket demarree",
        serde_json::json!({
            "event_type": "gateway.startup",
            "bind": config.bind_addr(),
            "max_connections": config.max_connections,
        }),
    );

    info!("Sentinel Gateway pret (WebSocket sur /ws)");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Erreur serveur");

    // Graceful shutdown avec timeout
    let timeout = std::time::Duration::from_secs(config.shutdown_timeout_secs);
    info!(
        timeout_secs = config.shutdown_timeout_secs,
        "Arret en cours, attente des connexions..."
    );
    tokio::time::sleep(timeout).await;

    gw_logger.warn(
        "Gateway WebSocket arretee",
        serde_json::json!({"event_type": "gateway.shutdown"}),
    );

    info!("Sentinel Gateway arrete proprement");
}

/// Origines exactes declarees dans `ALLOWED_ORIGINS`. Vide ou `*` -> liste vide,
/// c'est-a-dire aucune restriction — reserve au developpement.
fn parse_origins(allowed_origins: &str) -> Vec<String> {
    if allowed_origins.is_empty() || allowed_origins == "*" {
        warn!(
            "ALLOWED_ORIGINS non configure ou en wildcard : le handshake WebSocket \
             n'est PAS filtre par origine. Lister les origines exactes en production."
        );
        return Vec::new();
    }
    allowed_origins
        .split(',')
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .map(str::to_owned)
        .collect()
}

fn build_cors(allowed_origins: &str, max_age_secs: u64) -> CorsLayer {
    let allow_origin =
        if allowed_origins.is_empty() || allowed_origins == "*" {
            AllowOrigin::any()
        } else {
            let origins: Vec<HeaderValue> =
                allowed_origins
                    .split(',')
                    .filter_map(|o| {
                        let trimmed = o.trim();
                        trimmed.parse().map_err(|e| {
                    warn!(origin = %trimmed, error = %e, "CORS origin invalide, ignore");
                    e
                }).ok()
                    })
                    .collect();
            AllowOrigin::list(origins)
        };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::HeaderName::from_static("x-request-id"),
        ])
        .max_age(std::time::Duration::from_secs(max_age_secs))
}
