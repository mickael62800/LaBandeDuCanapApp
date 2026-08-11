//! sentinel-worker — orchestrateur unifie des jobs periodiques
//! DiscordSentinel.
//!
//! Pourquoi ce binaire ?
//! Avant la fusion, chaque domaine avait son propre worker (ai-worker,
//! analytics-worker, cleanup-worker, etc. — 16 binaires). La plupart ne
//! faisaient qu'un simple `loop { sleep N; query DB; do action; }` :
//! 16 runtimes Tokio, 16 pools Postgres, 16 connexions Redis, 16 images
//! Docker, 16 healthchecks. Surcout RAM/ops important pour aucun
//! benefice d'isolation pratique (ils crashaient ensemble en cas de
//! coupure DB/Redis de toute facon).
//!
//! Cette crate fusionne tous ces jobs dans un binaire unique, organise
//! par **domaine** (`src/domains/{domain}/{job}.rs`). Chaque job reste
//! une fonction independante `run(deps) -> Result`, schedulee par
//! `scheduler.rs`. Le code metier ne change pas — seul le packaging
//! evolue.
//!
//! Migration progressive : les anciens workers continuent de tourner
//! pendant qu'on absorbe leurs jobs ici, puis on decommissione leurs
//! conteneurs un par un.

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

mod config;
mod context;
mod domains;
mod grpc;
mod scheduler;

use std::time::Duration;

use tokio::sync::watch;
use tracing::info;

use crate::config::WorkerConfig;
use crate::context::WorkerContext;

const WORKER_NAME: &str = "sentinel-worker";
const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 30;

#[tokio::main]
async fn main() {
    platform_common_worker::init_tracing("sentinel_worker=info");
    platform_common_worker::metrics::init_observability(WORKER_NAME);

    // Sentinel est la seule plateforme dont l'API expose `POST /api/logs` :
    // c'est donc la seule a activer le push des logs de jobs. Sans cet appel,
    // le socle s'en tient au log local — ce que font nexus-worker et
    // atrium-worker, dont l'API n'a pas cette route.
    platform_common_worker::enable_worker_log_push(
        std::env::var("SENTINEL_API_KEY").unwrap_or_default(),
    );

    let mut config = WorkerConfig::from_env();
    info!("Demarrage de Sentinel Worker (orchestrateur unifie)");

    let pg_pool = platform_common_worker::create_pg_pool(&config.database_url).await;
    info!("PostgreSQL connecte");

    let redis_client =
        platform_common_worker::redis_helpers::open_or_exit(config.redis_url.as_str());
    // Surcharge eventuelle depuis bot_guild_config (config dynamique).
    let db_config = platform_common_worker::load_worker_config(&pg_pool, WORKER_NAME).await;
    if !db_config.is_empty() {
        config.apply_db_config(&db_config);
        info!(keys = db_config.len(), "Config DB chargee");
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let context = match WorkerContext::new(
        config,
        pg_pool.clone(),
        redis_client,
        shutdown_rx.clone(),
    )
    .await
    {
        Ok(context) => context,
        Err(error) => {
            tracing::error!(%error, "Initialisation du contexte Worker impossible");
            std::process::exit(1);
        }
    };
    info!("Redis et clients HTTP partages initialises");

    let mut tasks = scheduler::start(context.clone());
    tasks.push(platform_common_worker::start_heartbeat(
        context.http.standard.clone(),
        context.config.api_url.clone(),
        WORKER_NAME,
        shutdown_rx,
    ));

    platform_common_worker::send_lifecycle_log(
        &context.config.api_url,
        WORKER_NAME,
        "info",
        "Sentinel Worker demarre",
    )
    .await;

    info!("Sentinel Worker pret");

    platform_common_worker::shutdown_signal().await;

    platform_common_worker::send_lifecycle_log(
        &context.config.api_url,
        WORKER_NAME,
        "warn",
        "Sentinel Worker en cours d'arret",
    )
    .await;

    info!("Arret en cours...");
    let _ = shutdown_tx.send(true);

    let shutdown_timeout = Duration::from_secs(platform_common_worker::env_u64(
        "WORKER_SHUTDOWN_TIMEOUT_SECS",
        DEFAULT_SHUTDOWN_TIMEOUT_SECS,
    ));
    let report = platform_common_worker::wait_for_tasks(tasks, shutdown_timeout).await;
    info!(
        completed = report.completed,
        aborted = report.aborted,
        join_errors = report.join_errors,
        "Taches du worker arretees"
    );

    // Le dernier ConnectionManager Redis est depose avant la fermeture du
    // pool PostgreSQL, une fois tous les jobs termines ou annules.
    let api_url = context.config.api_url.clone();
    drop(context);
    pg_pool.close().await;

    platform_common_worker::send_lifecycle_log(
        &api_url,
        WORKER_NAME,
        "info",
        "Sentinel Worker arrete",
    )
    .await;
    info!("Sentinel Worker arrete proprement");
}
