//! Point d'entree de l'API d'exploitation.

use std::sync::Arc;

use ops_api::{router, AppConfig, AppState};

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    // Le recorder doit etre installe AVANT le routeur : une metrique emise
    // avant lui est perdue.
    platform_common_api::metrics::init_prometheus();

    let config = match AppConfig::from_env() {
        Ok(config) => Arc::new(config),
        Err(error) => {
            tracing::error!(%error, "configuration invalide");
            std::process::exit(1);
        }
    };

    // `connect_lazy` : l'API demarre meme si Postgres n'est pas encore pret, et
    // se connecte a la premiere requete. Le healthcheck du conteneur reste vert
    // pendant le demarrage de la base, ce qui evite un cycle de redemarrages.
    let pool = match sqlx::PgPool::connect_lazy(&config.database_url) {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "URL de base invalide");
            std::process::exit(1);
        }
    };

    let alert_rules_uc = Arc::new(
        ops_core::application::manage_alert_rules_service::ManageAlertRulesService::new(Arc::new(
            ops_api::adapters::alert_rule_repository::PgAlertRuleRepository::new(pool.clone()),
        )),
    );

    let bind = config.bind_addr;
    let state = AppState {
        config,
        alert_rules_uc,
    };

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .expect("bind impossible");
    tracing::info!(%bind, "ops-api demarre");
    axum::serve(listener, router(state))
        .await
        .expect("serveur arrete");
}
