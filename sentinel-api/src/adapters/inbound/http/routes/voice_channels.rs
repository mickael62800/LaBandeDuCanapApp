//! Routes voice channels (salons vocaux dynamiques, whitelists, bans, invites, themes).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn voice_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/_all",
            get(handlers::community::voice_channels::list_all_channels),
        )
        .route(
            "/{guild_id}",
            get(handlers::community::voice_channels::list_channels),
        )
        .route(
            "/{guild_id}/history",
            get(handlers::community::voice_channels::list_history_channels)
                .delete(handlers::community::voice_channels::purge_history),
        )
        .route(
            "/",
            post(handlers::community::voice_channels::create_channel),
        )
        .route(
            "/by-channel/{channel_id}",
            get(handlers::community::voice_channels::get_channel_detail)
                .patch(handlers::community::voice_channels::update_channel)
                .delete(handlers::community::voice_channels::delete_channel),
        )
        .route(
            "/by-channel/{channel_id}/close",
            patch(handlers::community::voice_channels::close_channel),
        )
        .route(
            "/by-channel/{channel_id}/events",
            get(handlers::community::voice_channels::list_channel_events),
        )
        .route(
            "/by-channel/{channel_id}/purge",
            delete(handlers::community::voice_channels::purge_channel),
        )
        .route(
            "/by-channel/{channel_id}/transfer",
            patch(handlers::community::voice_channels::transfer_ownership),
        )
        .route(
            "/by-channel/{channel_id}/co-admins",
            post(handlers::community::voice_channels::add_co_admin),
        )
        .route(
            "/by-channel/{channel_id}/co-admins/{user_id}",
            delete(handlers::community::voice_channels::remove_co_admin),
        )
        .route(
            "/whitelist/{guild_id}/{owner_id}",
            get(handlers::community::voice_channels::get_whitelist),
        )
        .route(
            "/whitelist",
            post(handlers::community::voice_channels::add_to_whitelist),
        )
        .route(
            "/whitelist/{guild_id}/{owner_id}/{target_id}",
            delete(handlers::community::voice_channels::remove_from_whitelist),
        )
        .route(
            "/by-channel/{channel_id}/bans",
            post(handlers::community::voice_channels::ban_from_channel),
        )
        .route(
            "/by-channel/{channel_id}/bans/{user_id}",
            delete(handlers::community::voice_channels::unban_from_channel)
                .get(handlers::community::voice_channels::check_ban),
        )
        // Invite Links
        .route(
            "/by-channel/{channel_id}/invites",
            get(handlers::community::voice_channels::list_invite_links)
                .post(handlers::community::voice_channels::create_invite_link),
        )
        .route(
            "/by-channel/{channel_id}/invites/{link_id}",
            delete(handlers::community::voice_channels::revoke_invite_link),
        )
        .route(
            "/invites/{code}/use",
            post(handlers::community::voice_channels::use_invite_link),
        )
        // Themes
        .route(
            "/themes/{guild_id}",
            get(handlers::community::voice_channels::list_themes)
                .post(handlers::community::voice_channels::create_theme),
        )
        .route(
            "/themes/{guild_id}/{theme_id}",
            patch(handlers::community::voice_channels::update_theme)
                .delete(handlers::community::voice_channels::delete_theme),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/voice-channels", voice_inner())
}
