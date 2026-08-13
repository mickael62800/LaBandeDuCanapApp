//! Infrastructure et configuration partagees par les adaptateurs entrants.
//!
//! Ce sous-etat contient uniquement les capacites transversales qui ne
//! relevent d'aucun domaine metier. Il evite de remettre des champs plats dans
//! `AppState` tout en gardant des extracteurs Axum et le serveur gRPC legers.

use std::sync::Arc;

use axum::extract::FromRef;
use platform_core::ops::ports::outbound::log_repository::LogRepository;

use crate::sentinel::adapters::outbound::nexus_games::NexusGamesClient;
use crate::sentinel::adapters::outbound::redis_cache::RedisCache;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;

use super::AppState;

#[derive(Clone)]
pub struct SharedState {
    pub log_repo: Arc<dyn LogRepository>,
    pub broadcaster: Arc<EventBroadcaster>,
    pub redis_client: redis::Client,
    pub cache: Option<Arc<RedisCache>>,
    pub nexus_games: Arc<NexusGamesClient>,
    pub api_key: String,
    pub guild_id: String,
    pub metrics_token: String,
    pub auth: Arc<platform_common_api::auth_client::AuthClient>,
}

impl FromRef<AppState> for SharedState {
    fn from_ref(state: &AppState) -> Self {
        state.shared.clone()
    }
}
