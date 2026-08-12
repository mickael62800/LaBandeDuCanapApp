//! # nexus-worker — worker de fond de la plateforme jeux Nexus
//!
//! Ne touche ni a la base ni a Docker : il POST periodiquement sur les
//! endpoints internes de nexus-api (`/api/games/internal/jobs/{job}`), qui
//! porte toute la logique metier. Config simple par variables d'environnement.

mod game_portal;
mod grand_salon;

use std::time::Duration;

use platform_common_worker::http_job::HttpJobClient;

struct NexusWorkerConfig {
    api_url: String,
    api_key: String,
    game_portal_intervals: game_portal::GamePortalIntervals,
}

impl NexusWorkerConfig {
    fn from_env() -> Self {
        Self {
            api_url: std::env::var("NEXUS_API_URL")
                .unwrap_or_else(|_| "http://localhost:3100".to_string()),
            api_key: std::env::var("NEXUS_API_KEY").unwrap_or_default(),
            game_portal_intervals: game_portal::GamePortalIntervals {
                health_check_secs: platform_common_worker::env_u64(
                    "GAME_HEALTH_CHECK_INTERVAL_SECS",
                    30,
                ),
                idle_shutdown_secs: platform_common_worker::env_u64(
                    "GAME_IDLE_SHUTDOWN_CHECK_INTERVAL_SECS",
                    3600,
                ),
                reconciler_secs: platform_common_worker::env_u64(
                    "GAME_RECONCILER_INTERVAL_SECS",
                    3600,
                ),
                image_cleanup_secs: platform_common_worker::env_u64(
                    "GAME_IMAGE_CLEANUP_INTERVAL_SECS",
                    86400,
                ),
                reveal_ip_secs: platform_common_worker::env_u64(
                    "GAME_REVEAL_IP_INTERVAL_SECS",
                    300,
                ),
                daily_ping_secs: platform_common_worker::env_u64(
                    "GAME_DAILY_PING_INTERVAL_SECS",
                    3600,
                ),
                // Court devant les 5 min de PREP_LEAD_MINUTES : sinon on
                // demarrerait le conteneur en retard sur l'ouverture.
                auto_start_secs: platform_common_worker::env_u64(
                    "GAME_AUTO_START_INTERVAL_SECS",
                    60,
                ),
            },
        }
    }
}

#[tokio::main]
async fn main() {
    platform_common_worker::init_tracing("nexus_worker=info");
    platform_common_worker::metrics::init_observability("nexus-worker");

    let config = NexusWorkerConfig::from_env();
    if config.api_key.is_empty() {
        tracing::warn!("NEXUS_API_KEY absente — les appels internes partiront sans Authorization");
    }

    let client = HttpJobClient::new(
        config.api_url.clone(),
        config.api_key,
        Duration::from_secs(30),
    );

    tracing::info!(api_url = %config.api_url, "nexus-worker demarre — jobs actifs");
    game_portal::start(client.clone(), config.api_url, config.game_portal_intervals);
    grand_salon::start(client);

    platform_common_worker::shutdown_signal().await;
    tracing::info!("signal recu — arret de nexus-worker");
}
