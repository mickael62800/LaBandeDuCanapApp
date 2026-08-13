//! Routes de la boite a idees (montees sous `/api/ideas`).

use axum::routing::{get, patch, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn idea_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::community::ideas::list_ideas)
                .post(handlers::community::ideas::create_idea),
        )
        // Place avant "/{id}" : sinon "by-channel" et "quota" seraient captures
        // comme des UUID et rejetes.
        .route(
            "/by-channel/{channel_id}",
            get(handlers::community::ideas::get_idea_by_channel),
        )
        .route(
            "/quota/{guild_id}/{author_id}",
            get(handlers::community::ideas::get_idea_quota),
        )
        .route(
            "/{id}",
            get(handlers::community::ideas::get_idea)
                .delete(handlers::community::ideas::delete_idea),
        )
        .route(
            "/{id}/status",
            patch(handlers::community::ideas::decide_idea),
        )
        .route(
            "/{id}/channel",
            patch(handlers::community::ideas::set_idea_channel),
        )
        .route(
            "/{id}/messages",
            post(handlers::community::ideas::add_idea_message),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/ideas", idea_inner())
}
