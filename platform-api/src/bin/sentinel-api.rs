// Phase 1 — Quick wins : jemalloc en allocateur global (Linux/macOS).
// Sur Windows MSVC, on retombe sur l'allocateur système (jemalloc ne compile
// pas dans ce target). Gain typique : -15 % RAM résidente sur les processus
// long-running grâce à une meilleure gestion de la fragmentation mémoire.
#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

use std::time::Duration;

use platform_api::sentinel::adapters::inbound::http::router;
use platform_api::sentinel::adapters::inbound::http::state::AppState;
use platform_api::sentinel::bootstrap;
use platform_api::sentinel::config::AppConfig;
use sqlx::PgPool;
use tokio::signal;
use tracing::info;
use tracing::warn;

#[tokio::main]
async fn main() {
    run().await;
}

pub async fn run() {
    dotenvy::dotenv().ok();

    // Fixe le t0 pour l'uptime expose via /api/system/info.
    platform_api::sentinel::adapters::inbound::http::handlers::system::info::record_startup();

    if std::env::var_os("PLATFORM_API_UNIFIED_RUNTIME").is_none() {
        init_tracing();
    }

    // Phase 0 — Observabilité : installe le recorder Prometheus AVANT toute
    // émission de métriques. Doit être appelé avant `Router::build`.
    platform_api::sentinel::adapters::inbound::http::metrics::init_prometheus();

    // Échantillonnage du runtime tokio toutes les 10s → gauges Prometheus
    // (workers_count, busy_ratio, queue_depth, ...).
    platform_api::sentinel::adapters::inbound::http::metrics::spawn_tokio_runtime_sampler();

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
    verifier_mono_serveur(&pg_pool).await;

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
    sqlx::migrate!("./migrations/sentinel")
        .run(pool)
        .await
        .expect("Erreur lors des migrations");
    info!("Migrations appliquées");
}

/// Declencheur du point S2 (`SECURITE-POINTS-OUVERTS.md`), verifie au boot.
///
/// POURQUOI CETTE REQUETE EXISTE
///
/// Le verrou mono-serveur (`middleware/single_guild.rs`) ne lit que l'URL :
/// une trentaine de handlers recoivent leur `guild_id` dans le CORPS et passent
/// donc sans etre confrontes a `GUILD_ID`. C'est un arbitrage assume, et il
/// tient a UNE condition : l'installation ne sert qu'une guilde, donc un
/// `guild_id` etranger dans un corps ne designe aucune donnee existante.
///
/// Cette condition est ecrite dans le document d'audit — mais un document ne
/// previent personne le jour ou elle cesse d'etre vraie. Cette requete, elle,
/// le dit au premier redemarrage.
///
/// Une seule requete, au demarrage, sur une table qui compte quelques lignes.
/// Son echec n'empeche PAS de demarrer : c'est une sonde, pas une dependance —
/// refuser de servir parce qu'un compte de controle n'a pas abouti serait une
/// panne creee par le garde-fou lui-meme.
async fn verifier_mono_serveur(pool: &PgPool) {
    let guildes: Option<i64> = match sqlx::query_scalar("SELECT COUNT(*) FROM guilds")
        .fetch_one(pool)
        .await
    {
        Ok(n) => Some(n),
        Err(e) => {
            warn!(error = %e, "verification mono-serveur impossible (sonde S2)");
            None
        }
    };

    if let Some(n) = guildes {
        if n > 1 {
            tracing::error!(
                guildes = n,
                "La base contient PLUSIEURS guildes : le point S2 (guild_id de \
                 corps hors du verrou mono-serveur) n'est plus theorique. Une \
                 trentaine de handlers acceptent un guild_id de corps sans le \
                 confronter a la configuration. Voir SECURITE-POINTS-OUVERTS.md"
            );
        }
    }
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
        if let Err(error) =
            platform_api::sentinel::adapters::inbound::grpc::server::serve_grpc(state, grpc_addr)
                .await
        {
            tracing::error!(%error, "Arret: le serveur gRPC ne peut pas demarrer de facon sure");
            std::process::exit(1);
        }
    });
}

/// Bind Axum + journalisation systeme du cycle de vie + graceful shutdown.
async fn serve_http(state: AppState, config: &AppConfig, pg_pool: PgPool) {
    let api_log_repo = state.shared.log_repo.clone();
    let api_log_redis = state.shared.redis_client.clone();

    let app = router::build(
        state,
        config.max_body_size,
        config.rate_limit_per_sec,
        &config.allowed_origins,
    );

    let listener = tokio::net::TcpListener::bind(config.bind_addr())
        .await
        .expect("Impossible de bind le port");

    // Log de demarrage dans la stream lue par la colonne API du dashboard.
    {
        let entry = platform_core::ops::domain::entities::log_entry::LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: "info".into(),
            bot: "sentinel-api".into(),
            server: String::new(),
            message: format!("API demarree sur {}", config.bind_addr()),
            category: "api".into(),
            details: serde_json::json!({"event": "startup", "bind": config.bind_addr()}),
        };
        platform_api::sentinel::adapters::outbound::system::redis_log_stream::xadd_log(
            &api_log_redis,
            &entry,
        )
        .await;
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

    // Log d'arret : Redis pour l'affichage, Postgres car niveau warn.
    {
        let entry = platform_core::ops::domain::entities::log_entry::LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: "warn".into(),
            bot: "sentinel-api".into(),
            server: String::new(),
            message: "API en cours d'arret".into(),
            category: "api".into(),
            details: serde_json::json!({"event": "shutdown"}),
        };
        platform_api::sentinel::adapters::outbound::system::redis_log_stream::xadd_log(
            &api_log_redis,
            &entry,
        )
        .await;
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
