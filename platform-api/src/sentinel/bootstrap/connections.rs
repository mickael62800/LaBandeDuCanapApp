//! Connexions infra : pool PostgreSQL (compat pgbouncer) + client Redis.

use std::str::FromStr;
use std::time::Duration;

use crate::sentinel::config::AppConfig;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgPoolOptions;
use tracing::error;
use tracing::info;

/// Connecte a PostgreSQL avec pgbouncer transaction pooling compat.
///
/// Phase 7A opt C.1 : compat pgbouncer transaction pooling.
///
/// `.statement_cache_capacity(0)` : pgbouncer en transaction pooling ne
///   garantit pas que deux requetes consecutives arrivent sur la meme
///   backend connection, donc les prepared statements caches par sqlx
///   (via son cache LRU par defaut) peuvent etre invalides silencieusement
///   et cela declenche `query_wait_timeout` (code 08P01). Desactiver le
///   cache resout le probleme — cout CPU marginal.
///
/// `.application_name("sentinel-api")` : permet a pgbouncer/postgres de
///   tracer les connexions par service (visible dans `pg_stat_activity`).
pub async fn connect_pg(config: &AppConfig) -> sqlx::PgPool {
    let connect_opts = PgConnectOptions::from_str(&config.database_url)
        .expect("DATABASE_URL invalide")
        .statement_cache_capacity(0)
        .application_name("sentinel-api");

    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(10))
        .test_before_acquire(false)
        .connect_with(connect_opts)
        .await
        .expect("Impossible de se connecter a PostgreSQL")
}

/// Ouvre le client Redis + purge cache bot:definitions + check liveness.
///
/// Purger le cache des definitions de bots apres migration : les migrations
/// peuvent modifier les config_schema (ex: 113 = ajout des 4 salons audit),
/// mais le cache Redis bot:definitions a un TTL d'1h. Sans ca, les changements
/// n'apparaissent qu'apres expiration du TTL.
pub async fn connect_redis(config: &AppConfig) -> redis::Client {
    let redis_client = redis::Client::open(config.redis_url.as_str()).expect("URL Redis invalide");

    if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
        use redis::AsyncCommands;
        let _: Result<(), _> = conn.del::<_, ()>("bot:definitions").await;
        info!("Cache Redis bot:definitions purge (post-migration)");
    }

    // Verifier la connexion Redis au demarrage
    match redis_client.get_multiplexed_async_connection().await {
        Ok(_) => info!("Redis connecte"),
        Err(e) => error!("Redis indisponible au demarrage: {e} — le cache sera desactive"),
    }

    redis_client
}
