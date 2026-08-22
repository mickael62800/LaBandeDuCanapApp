//! Routes Community (decisions server-side) montees sous `/api/community`.
//! Actuellement : eligibilite de role + validation de parrainage.

use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route(
            "/eligibility/{guild_id}/role",
            post(handlers::community::eligibility::check_role_eligibility),
        )
        .route(
            "/eligibility/{guild_id}/sponsorship",
            post(handlers::community::eligibility::validate_sponsorship),
        )
        // Delai d'acceptation du reglement. Deux POST plutot qu'un POST et un
        // DELETE : le bot n'emet plus aucun DELETE HTTP.
        .route(
            "/rules-deadline/start",
            post(handlers::community::rules_deadline::start_rules_deadline),
        )
        .route(
            "/rules-deadline/clear",
            post(handlers::community::rules_deadline::clear_rules_deadline),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/community", inner())
}
