//! Runtime de transition : les quatre surfaces historiques partagent un seul
//! processus tout en conservant leurs listeners et leurs ports.

#[allow(dead_code)]
#[path = "../runtime/atrium.rs"]
mod atrium_runtime;
#[allow(dead_code)]
#[path = "../runtime/nexus.rs"]
mod nexus_runtime;
#[allow(dead_code)]
#[path = "../runtime/ops.rs"]
mod ops_runtime;
#[allow(dead_code)]
#[path = "../runtime/sentinel.rs"]
mod sentinel_runtime;

use std::time::Duration;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    // Les points d'entree de compatibilite restent directement executables.
    // Dans ce runtime, le tracing et les metriques sont initialises une fois.
    std::env::set_var("PLATFORM_API_UNIFIED_RUNTIME", "1");
    init_tracing();
    platform_api::shared::metrics::init_prometheus();
    platform_api::shared::metrics::spawn_tokio_runtime_sampler();

    tracing::info!("demarrage du runtime platform-api sur quatre listeners");

    let mut services = tokio::task::JoinSet::new();
    services.spawn(async { ("sentinel", sentinel_runtime::run().await) });
    services.spawn(async { ("ops", ops_runtime::run().await) });
    services.spawn(async { ("nexus", nexus_runtime::run().await) });
    services.spawn(async { ("atrium", atrium_runtime::run().await) });

    tokio::select! {
        () = shutdown_signal() => {
            tracing::info!("signal d'arret recu; attente des quatre surfaces");
        }
        completed = services.join_next() => {
            match completed {
                Some(Ok((name, ()))) => tracing::error!(service = name, "surface arretee sans signal global"),
                Some(Err(error)) => tracing::error!(%error, "echec d'une surface API"),
                None => tracing::error!("aucune surface API n'a demarre"),
            }
            services.abort_all();
            std::process::exit(1);
        }
    }

    let graceful = async {
        while let Some(result) = services.join_next().await {
            match result {
                Ok((name, ())) => tracing::info!(service = name, "surface arretee"),
                Err(error) if error.is_cancelled() => {}
                Err(error) => tracing::error!(%error, "echec pendant l'arret d'une surface"),
            }
        }
    };

    if tokio::time::timeout(Duration::from_secs(shutdown_timeout_secs()), graceful)
        .await
        .is_err()
    {
        tracing::error!("delai d'arret depasse; interruption des surfaces restantes");
        services.abort_all();
    }
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "platform_api=info,tower_http=info".into());
    let json = std::env::var("LOG_FORMAT").is_ok_and(|value| value == "json");
    if json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_span_list(true)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    }
}

fn shutdown_timeout_secs() -> u64 {
    std::env::var("PLATFORM_API_SHUTDOWN_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(30)
}

async fn shutdown_signal() {
    let ctrl_c = async { tokio::signal::ctrl_c().await.expect("ecoute Ctrl+C") };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
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
