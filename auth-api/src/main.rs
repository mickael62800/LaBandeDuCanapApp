//! API d'identité : OAuth2 Discord, sessions web, gate superadmin.
//!
//! Source de vérité unique pour les trois plateformes Discord et pour
//! l'exploitation. Elle n'est pas publiée sur l'hôte : le SPA l'atteint par la
//! passerelle nginx, les autres services par le réseau interne avec
//! `AUTH_API_TOKEN`.

use std::sync::Arc;

use auth_core::application::manage_session_service::ManageSessionService;
use auth_core::application::resolve_access_service::ResolveAccessService;
use auth_core::ports::outbound::session_repository::SessionRepository;
use sqlx::postgres::PgPoolOptions;

mod adapters;
mod config;
mod http;

use adapters::discord_http::HttpDiscordIdentity;
use adapters::postgres_sessions::PgSessionRepository;
use adapters::redis_stores::{cache_key, RedisIdentityCache, RedisLoginStateStore};
use config::AppConfig;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Arc::new(AppConfig::from_env());

    if config.api_token.trim().is_empty() {
        // On ne refuse pas de démarrer : le flux OAuth (`/auth/discord/*`) et le
        // healthcheck doivent rester servis, et le développement local en a
        // besoin. Les routes de SERVICE, elles, refusent désormais (503) au lieu
        // de laisser passer — `/access` résout n'importe quel jeton et
        // `/security/last-logins` expose l'historique de connexion des
        // administrateurs. Cf. `http::authorize_service`.
        tracing::warn!(
            "AUTH_API_TOKEN vide — les routes de service (/access, /security/*) repondront 503. \
             Les consommateurs (sentinel-api, ops-api, gateway, nginx) verront une identite \
             indisponible tant que le jeton n'est pas defini."
        );
    }

    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "connexion PostgreSQL impossible");
            std::process::exit(1);
        }
    };

    if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
        tracing::error!(%error, "migrations de l'identite en echec");
        std::process::exit(1);
    }

    let redis = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => client,
        Err(error) => {
            tracing::error!(%error, "REDIS_URL invalide");
            std::process::exit(1);
        }
    };

    let discord = Arc::new(HttpDiscordIdentity::new(
        config.discord_client_id.clone(),
        config.discord_client_secret.clone(),
        config.discord_redirect_uri.clone(),
    ));
    let discord_configured = discord.is_configured();
    if !discord_configured {
        tracing::warn!(
            "OAuth Discord non configure — /auth/discord/authorize repondra 503 (les routes de service restent utilisables)"
        );
    }

    let sessions = Arc::new(PgSessionRepository::new(pool));

    // Le filtre SQL refuse deja une session expiree au moment ou elle est
    // presentee. Cette tache traite aussi celles qui ne seront plus jamais
    // presentees et empeche la table de croitre indefiniment.
    let cleanup_sessions = sessions.clone();
    tokio::spawn(async move {
        loop {
            match cleanup_sessions.purge_expired().await {
                Ok(count) if count > 0 => {
                    tracing::info!(count, "sessions OAuth expirees supprimees");
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(%error, "purge des sessions OAuth impossible"),
            }
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    });

    let state = Arc::new(http::AppState {
        sessions: Arc::new(ManageSessionService {
            sessions: sessions.clone(),
            discord: discord.clone(),
            states: Arc::new(RedisLoginStateStore::new(redis.clone())),
            policy: config.superadmins.clone(),
            new_state: || uuid::Uuid::new_v4().to_string(),
        }),
        access: Arc::new(ResolveAccessService {
            discord,
            cache: Arc::new(RedisIdentityCache::new(redis)),
            policy: config.superadmins.clone(),
            cache_key,
        }),
        discord_configured,
        config: config.clone(),
    });

    let listener = match tokio::net::TcpListener::bind(config.bind_addr).await {
        Ok(l) => l,
        Err(error) => {
            tracing::error!(%error, addr = %config.bind_addr, "bind impossible");
            std::process::exit(1);
        }
    };
    tracing::info!(addr = %config.bind_addr, "auth-api demarre");

    if let Err(error) = axum::serve(
        listener,
        http::router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    {
        tracing::error!(%error, "serveur arrete");
        std::process::exit(1);
    }
}
