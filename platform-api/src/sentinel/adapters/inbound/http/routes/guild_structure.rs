//! Routes du constructeur de salons (montees sous `/api/guild-structure`).

use axum::routing::{delete, get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}",
            get(handlers::guild_structure::plan::get_structure),
        )
        .route(
            "/{guild_id}/roles",
            get(handlers::guild_structure::plan::list_roles),
        )
        .route(
            "/{guild_id}/apply",
            post(handlers::guild_structure::plan::apply_plan),
        )
        .route(
            "/{guild_id}/channels/{channel_id}",
            delete(handlers::guild_structure::plan::delete_channel),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/guild-structure", inner())
}
