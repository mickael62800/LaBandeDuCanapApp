//! Routes members (DB-backed + direct Discord API).

use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn member_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}",
            get(handlers::community::guild_members::list_members_db),
        )
        .route(
            "/{guild_id}/{user_id}",
            get(handlers::community::guild_members::get_member)
                .patch(handlers::community::guild_members::update_member)
                .delete(handlers::community::guild_members::remove_member),
        )
        .route(
            "/{guild_id}/{user_id}/summary",
            get(handlers::community::guild_members::get_member_summary),
        )
        .route(
            "/{guild_id}/{user_id}/reset",
            post(handlers::community::guild_members::reset_member),
        )
        // Lifecycle : appeles par sentinel-bot sur GuildMemberRemove/Add.
        .route(
            "/{guild_id}/{user_id}/leave",
            post(handlers::community::guild_members::leave_member),
        )
        .route(
            "/{guild_id}/{user_id}/rejoin",
            post(handlers::community::guild_members::rejoin_member),
        )
        .route(
            "/sync",
            post(handlers::community::guild_members::sync_members),
        )
        .route(
            "/register",
            post(handlers::community::guild_members::register_member),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Members (DB-backed)
        .nest("/api/members", member_inner())
        // Guild members (direct Discord API)
        .route(
            "/api/guilds/{guild_id}/members",
            get(handlers::community::guild_members::list_members),
        )
        // Guild text channels (direct Discord API, Phase 9 Part E)
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
