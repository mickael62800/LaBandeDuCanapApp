//! Routes guilds (direct Discord API endpoints for dashboard).

use axum::routing::get;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Guild text channels (direct Discord API)
        .route(
            "/api/guilds/{guild_id}/channels",
            get(handlers::community::guild_channels::list_text_channels),
        )
        .route(
            "/api/guilds/{guild_id}/channels/all",
            get(handlers::community::guild_channels::list_all_channels),
        )
        // Guild emojis (direct Discord API)
        .route(
            "/api/guilds/{guild_id}/emojis",
            get(handlers::community::guild_emojis::list_emojis),
        )
}
