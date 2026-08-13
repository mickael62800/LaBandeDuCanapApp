//! Routes bot persistence (endpoints fire-and-forget consommes par les bots).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Bot persistence (fire-and-forget endpoints for bot data)
        .route(
            "/api/name-history",
            post(handlers::system::bot_persistence::create_name_history),
        )
        .route(
            "/api/name-history/{guild_id}/{user_id}",
            get(handlers::system::bot_persistence::list_name_history),
        )
        .route(
            "/api/levels/{guild_id}/{user_id}/streak",
            patch(handlers::system::bot_persistence::update_streak),
        )
        .route(
            "/api/tickets/{id}/sla",
            patch(handlers::system::bot_persistence::update_ticket_sla),
        )
        .route(
            "/api/sponsorships",
            post(handlers::system::bot_persistence::create_sponsorship),
        )
        .route(
            "/api/sponsorships/{guild_id}",
            get(handlers::system::bot_persistence::list_sponsorships),
        )
        .route(
            "/api/temp-roles",
            post(handlers::system::bot_persistence::create_temp_role),
        )
        .route(
            "/api/temp-roles/{guild_id}",
            get(handlers::system::bot_persistence::list_temp_roles),
        )
        .route(
            "/api/temp-roles/{guild_id}/{user_id}/{role_id}",
            delete(handlers::system::bot_persistence::delete_temp_role),
        )
        .route(
            "/api/moderation/pending",
            post(handlers::system::bot_persistence::create_pending_action),
        )
        .route(
            "/api/moderation/pending/guild/{guild_id}",
            get(handlers::system::bot_persistence::list_pending_actions),
        )
        .route(
            "/api/moderation/pending/{id}/resolve",
            patch(handlers::system::bot_persistence::resolve_pending_action),
        )
}
