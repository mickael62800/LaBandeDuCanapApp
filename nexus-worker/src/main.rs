//! # nexus-worker — worker de fond de la plateforme jeux Nexus
//!
//! Ne touche ni a la base ni a Docker : il POST periodiquement sur les
//! endpoints internes de nexus-api (`/api/games/internal/jobs/{job}`), qui
//! porte toute la logique metier. Config simple par variables d'environnement.

mod game_portal;
mod grand_salon;

/// Lit une variable d'env u64, sinon retourne le defaut (valeur invalide
/// signalee en warn puis defaut applique).
fn env_u64(name: &str, default: u64) -> u64 {
    match std::env::var(name) {
        Ok(v) => v.parse().unwrap_or_else(|_| {
            tracing::warn!(var = name, value = %v, default, "valeur invalide, defaut applique");
            default
        }),
        Err(_) => default,
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let api_url =
        std::env::var("NEXUS_API_URL").unwrap_or_else(|_| "http://localhost:3100".to_string());
    if std::env::var("NEXUS_API_KEY")
        .unwrap_or_default()
        .is_empty()
    {
        tracing::warn!("NEXUS_API_KEY absente — les appels internes partiront sans Authorization");
    }

    let intervals = game_portal::GamePortalIntervals {
        health_check_secs: env_u64("GAME_HEALTH_CHECK_INTERVAL_SECS", 30),
        idle_shutdown_secs: env_u64("GAME_IDLE_SHUTDOWN_CHECK_INTERVAL_SECS", 3600),
        reconciler_secs: env_u64("GAME_RECONCILER_INTERVAL_SECS", 3600),
        image_cleanup_secs: env_u64("GAME_IMAGE_CLEANUP_INTERVAL_SECS", 86400),
        reveal_ip_secs: env_u64("GAME_REVEAL_IP_INTERVAL_SECS", 300),
        daily_ping_secs: env_u64("GAME_DAILY_PING_INTERVAL_SECS", 3600),
    };

    tracing::info!(api_url = %api_url, "nexus-worker demarre — jobs game-portal actifs");
    game_portal::start(api_url, intervals);
    grand_salon::start();

    if let Err(e) = tokio::signal::ctrl_c().await {
        tracing::error!("attente du signal impossible: {e}");
        return;
    }
    tracing::info!("signal recu — arret de nexus-worker");
}
