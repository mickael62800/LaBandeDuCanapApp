//! Planificateur unique et volontairement minimal.
//!
//! Ce processus connait uniquement des horaires et des endpoints HTTP. Il ne
//! depend d'aucun domaine metier et ne possede ni acces SQL/Redis, ni secret
//! Discord, ni privilege hote.

mod config;
mod domains;
mod http;
mod metrics;
mod schedule;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "platform_scheduler=info".into()),
        )
        .json()
        .init();
    metrics::init();

    let config = config::Config::from_env().unwrap_or_else(|error| {
        tracing::error!(%error, "configuration du scheduler invalide");
        std::process::exit(2);
    });

    let started = domains::start(&config);
    tracing::info!(domains = started, "platform-scheduler demarre");

    shutdown_signal().await;
    tracing::info!("platform-scheduler arrete");
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut terminate = signal(SignalKind::terminate()).expect("signal SIGTERM");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
