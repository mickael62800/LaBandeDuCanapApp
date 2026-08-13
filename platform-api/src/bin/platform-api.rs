//! Runtime de transition : les quatre surfaces historiques partagent un seul
//! processus tout en conservant leurs listeners et leurs ports.

#[allow(dead_code)]
#[path = "atrium-api.rs"]
mod atrium_api;
#[allow(dead_code)]
#[path = "nexus-api.rs"]
mod nexus_api;
#[allow(dead_code)]
#[path = "ops-api.rs"]
mod ops_api;
#[allow(dead_code)]
#[path = "sentinel-api.rs"]
mod sentinel_api;

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
    services.spawn(async { ("sentinel", sentinel_api::run().await) });
    services.spawn(async { ("ops", ops_api::run().await) });
    if domain_enabled(
        "NEXUS_API_ENABLED",
        "nexus",
        &["NEXUS_DATABASE_URL", "NEXUS_API_KEY"],
    ) {
        services.spawn(async { ("nexus", nexus_api::run().await) });
    }
    if domain_enabled(
        "ATRIUM_API_ENABLED",
        "atrium",
        &[
            "ATRIUM_RAG_DATABASE_URL",
            "ATRIUM_API_TOKEN",
            "ATRIUM_GRPC_TOKEN",
        ],
    ) {
        services.spawn(async { ("atrium", atrium_api::run().await) });
    }

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

fn domain_enabled(flag: &str, name: &str, required: &[&str]) -> bool {
    let requested = std::env::var(flag).is_ok_and(|value| value.eq_ignore_ascii_case("true"));
    let configured = required
        .iter()
        .all(|key| std::env::var(key).is_ok_and(|value| !value.trim().is_empty()));
    let enabled = requested && configured;
    if requested && !configured {
        tracing::error!(
            domain = name,
            "domaine demande mais configuration incomplete"
        );
        std::process::exit(1);
    }
    if enabled {
        tracing::info!(domain = name, "domaine optionnel active");
    } else {
        tracing::info!(
            domain = name,
            "domaine optionnel inactif (configuration absente)"
        );
    }
    enabled
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
