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
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/community", inner())
}
