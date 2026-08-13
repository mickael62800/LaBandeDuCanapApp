//! Routes bot-facing (rules, infractions) sans les endpoints lourds
//! d'inference deplacés dans `heavy`.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

/// Routes bot standard (sans les endpoints lourds d'inference deplacés dans heavy_routes).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/rules/{guild_id}",
            get(handlers::moderation::rules::get_rules),
        )
        .route("/rules", post(handlers::moderation::rules::create_rule))
        .route(
            "/rules/{guild_id}/{rule_id}",
            delete(handlers::moderation::rules::delete_rule),
        )
        .route(
            "/infractions/{guild_id}",
            get(handlers::moderation::infractions::list_infractions),
        )
        .route(
            "/infractions/delete/{id}",
            delete(handlers::moderation::infractions::delete_infraction),
        )
}
