//! Routes des panneaux de roles (`/api/role-panels`) et roles automatiques
//! (`/api/auto-roles`).
//!
//! Surface HTTP supprimee lors d'un nettoyage trop large ; le metier
//! (`ManageRolePanelsUseCase`) et les DTO avaient survecu. Les handlers passent
//! par `CommunityState.role_panels_uc`.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn role_panel_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::community::role_panels::create_panel))
        .route(
            "/{guild_id}",
            get(handlers::community::role_panels::list_panels),
        )
        .route(
            "/detail/{panel_id}",
            get(handlers::community::role_panels::get_panel)
                .delete(handlers::community::role_panels::delete_panel),
        )
        .route(
            "/by-message/{message_id}",
            get(handlers::community::role_panels::get_panel_by_message),
        )
        .route(
            "/set-message",
            patch(handlers::community::role_panels::set_message_id),
        )
}

fn auto_role_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}",
            get(handlers::community::role_panels::list_auto_roles),
        )
        .route("/", post(handlers::community::role_panels::add_auto_role))
        .route(
            "/{guild_id}/{role_id}",
            delete(handlers::community::role_panels::delete_auto_role),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/role-panels", role_panel_inner())
        .nest("/api/auto-roles", auto_role_inner())
}
