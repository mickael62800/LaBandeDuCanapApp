use atrium_api::{router, AppConfig};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    platform_common_api::metrics::init_prometheus();

    let config = AppConfig::from_env().expect("Configuration Atrium API invalide");
    // Un seul pool pour toute l'API : migrations, stores HTTP et gRPC en
    // partagent les connexions (cf. `atrium_api::connect_pool`).
    let pool = atrium_api::connect_pool(&config).expect("Pool PostgreSQL Atrium invalide");
    atrium_api::run_migrations(&pool)
        .await
        .expect("Erreur lors des migrations Atrium");
    let rag = atrium_api::rag::service(pool.clone(), &config);
    let budget = std::sync::Arc::new(atrium_api::budget::BudgetGuard::new(pool.clone(), &config));
    let control = std::sync::Arc::new(atrium_api::control::BotControlStore::new(
        pool.clone(),
        &config,
    ));
    let memory = std::sync::Arc::new(atrium_api::memory::ConversationMemory::new(pool.clone()));
    // Indexation RAG en arriere-plan : elle depend d'Ollama (embeddings), qui
    // peut etre indisponible au demarrage. La bloquer ici empechait toute l'API
    // de servir — accueil, admin, quotas — pour un service annexe. En cas
    // d'echec, le RAG renvoie simplement un contexte vide et une prochaine
    // execution rattrapera l'indexation ; le reste d'Atrium reste disponible.
    let index_rag = rag.clone();
    tokio::spawn(async move {
        if let Err(error) = index_rag.index_knowledge().await {
            tracing::error!(%error, "Indexation RAG Atrium impossible (contexte vide en attendant)");
        }
    });
    let addr = config.bind_addr;
    tokio::spawn(atrium_api::grpc::serve(
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
    tracing::info!(%addr, "Atrium API demarree");
    axum::serve(listener, router(config, pool, rag, budget, control, memory))
        .await
        .expect("Erreur serveur Atrium API");
}
