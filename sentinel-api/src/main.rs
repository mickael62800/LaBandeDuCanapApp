// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système (jemalloc ne compile
// pas dans ce target). Gain typique : -15 % RAM résidente sur les processus
// long-running grâce à une meilleure gestion de la fragmentation mémoire.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::time::Duration;

use sentinel_api::adapters::inbound::http::router;
use sentinel_api::adapters::inbound::http::state::AppState;
use sentinel_api::bootstrap;
use sentinel_api::config::AppConfig;
use sqlx::PgPool;
use tokio::signal;
use tracing::info;
use tracing::warn;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    // Fixe le t0 pour l'uptime expose via /api/system/info.
    sentinel_api::adapters::inbound::http::handlers::system::info::record_startup();

    init_tracing();

    // Phase 0 — Observabilité : installe le recorder Prometheus AVANT toute
    // émission de métriques. Doit être appelé avant `Router::build`.
    sentinel_api::adapters::inbound::http::metrics::init_prometheus();

    // Échantillonnage du runtime tokio toutes les 10s → gauges Prometheus
    // (workers_count, busy_ratio, queue_depth, ...).
    sentinel_api::adapters::inbound::http::metrics::spawn_tokio_runtime_sampler();

    let config = AppConfig::from_env();

    info!(
        addr = %config.bind_addr(),
        rate_limit = config.rate_limit_per_sec,
        max_body = config.max_body_size,
        "Démarrage de Sentinel API"
    );

    // ── Connexions infrastructure ──
    let pg_pool = bootstrap::connect_pg(&config).await;
    run_migrations(&pg_pool).await;

    let redis_client = bootstrap::connect_redis(&config).await;

    // Invalide le cache des definitions de bots au boot. Les definitions
    // viennent de migrations SQL (bot_definitions) — sans ca, l'ajout d'un
    // nouveau bot/worker via migration reste invisible jusqu'a expiration
    // du TTL Redis (1h).
    {
        use redis::AsyncCommands;
        if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
            let _ = conn.del::<_, ()>("bot:definitions").await;
        }
    }

    let state = bootstrap::build_app_state(&config, pg_pool.clone(), redis_client).await;
    bootstrap::spawn_security_workers(&state);

    spawn_grpc_server(state.clone(), &config);
    serve_http(state, &config, pg_pool).await;
}

/// Configure tracing-subscriber (JSON en prod, pretty en dev).
fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "sentinel_api=info,tower_http=debug".into());

    // JSON structuré en production, format lisible en dev
    let json_logs = std::env::var("LOG_FORMAT")
        .map(|v| v == "json")
        .unwrap_or(false);

    if json_logs {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_span_list(true)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }
}

/// Applique les migrations sqlx au boot.
async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Erreur lors des migrations");
    info!("Migrations appliquées");
}

/// Phase 7A — gRPC interne (tonic) en parallele d'Axum.
/// Coexistence sur 2 ports : HTTP sur PORT, gRPC sur GRPC_PORT.
/// Les bots sont migres progressivement; HTTP reste actif tant qu'au
/// moins un consommateur n'est pas migre.
fn spawn_grpc_server(state: AppState, config: &AppConfig) {
    let grpc_addr: std::net::SocketAddr = config
        .grpc_bind_addr()
        .parse()
        .expect("GRPC_PORT/HOST invalide");
    tokio::spawn(async move {
        sentinel_api::adapters::inbound::grpc::server::serve_grpc(state, grpc_addr).await;
    });
}

/// Bind Axum + log startup/shutdown en BDD + graceful shutdown.
async fn serve_http(state: AppState, config: &AppConfig, pg_pool: PgPool) {
    let api_log_repo = state.log_repo.clone();

    let app = router::build(
        state,
        config.max_body_size,
        config.rate_limit_per_sec,
        &config.allowed_origins,
    );

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Impossible de bind le port");

    // Log demarrage en BDD
    {
        let entry = ops_core::domain::entities::log_entry::LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: "info".into(),
            bot: "sentinel-api".into(),
            server: String::new(),
            message: format!("API demarree sur {}", config.bind_addr()),
            category: "api".into(),
            details: serde_json::json!({"event": "startup", "bind": config.bind_addr()}),
        };
        if let Err(e) = api_log_repo.save(&entry).await {
            warn!(error = %e, "Echec sauvegarde log API");
        }
    }

    info!("Sentinel API prêt (WebSocket sur /ws)");

    // ── Graceful shutdown ──
    let shutdown_timeout = Duration::from_secs(config.shutdown_timeout_secs);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("Erreur serveur");

    // Attendre que les connexions en cours se terminent
    info!(
        timeout_secs = config.shutdown_timeout_secs,
        "Arrêt en cours, attente des requêtes en vol..."
    );
    tokio::time::sleep(shutdown_timeout).await;

    // Log arret en BDD
    {
        let entry = ops_core::domain::entities::log_entry::LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: "warn".into(),
            bot: "sentinel-api".into(),
            server: String::new(),
            message: "API en cours d'arret".into(),
            category: "api".into(),
            details: serde_json::json!({"event": "shutdown"}),
        };
        if let Err(e) = api_log_repo.save(&entry).await {
            warn!(error = %e, "Echec sauvegarde log API");
        }
    }

    pg_pool.close().await;
    info!("Sentinel API arrêté proprement");
}

/// Écoute SIGTERM (Docker/K8s) et Ctrl+C (dev local)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("Impossible d'écouter Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Impossible d'écouter SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Signal Ctrl+C reçu"),
        _ = terminate => info!("Signal SIGTERM reçu"),
    }
}
