//! Routes tickets (montees sous `/api/tickets`).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn ticket_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handlers::system::tickets::list_tickets)
                .post(handlers::system::tickets::create_ticket),
        )
        .route(
            "/bulk",
            delete(handlers::system::tickets::bulk_delete_tickets),
        )
        .route("/{id}", get(handlers::system::tickets::get_ticket_detail))
        .route(
            "/{id}/messages",
            post(handlers::system::tickets::reply_ticket),
        )
        .route(
            "/{id}/close",
            patch(handlers::system::tickets::close_ticket),
        )
        .route(
            "/{id}/assign",
            patch(handlers::system::tickets::assign_ticket),
        )
        .route(
            "/{id}/status",
            patch(handlers::system::tickets::update_status),
        )
        .route(
            "/{id}/channels",
            patch(handlers::system::tickets::update_ticket_channel),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/tickets", ticket_inner())
}
