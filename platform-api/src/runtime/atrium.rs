use platform_api::atrium::{self, AppConfig};

pub async fn run() {
    dotenvy::dotenv().ok();
    if std::env::var_os("PLATFORM_API_UNIFIED_RUNTIME").is_none() {
        tracing_subscriber::fmt::init();
    }
    platform_api::shared::metrics::init_prometheus();

    let config = AppConfig::from_env().expect("Configuration Atrium API invalide");
    let pool = atrium::connect_pool(&config).expect("Pool PostgreSQL Atrium invalide");
    atrium::run_migrations(&pool)
        .await
        .expect("Erreur lors des migrations Atrium");
    let rag = atrium::rag::service(pool.clone(), &config);
    let budget = std::sync::Arc::new(atrium::budget::BudgetGuard::new(pool.clone(), &config));
    let control = std::sync::Arc::new(atrium::control::BotControlStore::new(pool.clone(), &config));
    let memory = std::sync::Arc::new(atrium::memory::ConversationMemory::new(pool.clone()));

    let index_rag = rag.clone();
    tokio::spawn(async move {
        if let Err(error) = index_rag.index_knowledge().await {
            tracing::error!(%error, "Indexation RAG Atrium impossible (contexte vide en attendant)");
        }
    });

    let addr = config.bind_addr;
    tokio::spawn(atrium::grpc::serve(
        config.clone(),
        pool.clone(),
        rag.clone(),
        budget.clone(),
        control.clone(),
        memory.clone(),
    ));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Impossible de binder Atrium API");
    tracing::info!(%addr, "Atrium API demarree depuis platform-api");
    atrium::serve_with_shutdown(
        listener,
        atrium::router(config, pool, rag, budget, control, memory),
        shutdown_signal(),
    )
    .await
    .expect("Erreur serveur Atrium API");
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
