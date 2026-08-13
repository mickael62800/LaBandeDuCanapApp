//! Routes de sauvegarde / restauration de serveur (montees sous
//! `/api/guild-backup`).

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}/snapshots",
            post(handlers::guild_backup::snapshots::store_snapshot)
                .get(handlers::guild_backup::snapshots::list_snapshots),
        )
        .route(
            "/snapshots/{snapshot_id}",
            get(handlers::guild_backup::snapshots::get_snapshot)
                .patch(handlers::guild_backup::snapshots::rename_snapshot)
                .delete(handlers::guild_backup::snapshots::delete_snapshot),
        )
        // Declencheurs : publient un event Redis consomme par le bot (le web ne
        // peut pas agir sur Discord).
        .route(
            "/{guild_id}/capture",
            post(handlers::guild_backup::snapshots::request_capture),
        )
        .route(
            "/snapshots/{snapshot_id}/restore",
            post(handlers::guild_backup::snapshots::request_restore),
        )
        // Re-attribution des roles aux membres de retour (pending_role_grants).
        .route(
            "/{guild_id}/pending-roles",
            post(handlers::guild_backup::pending_roles::save_pending_roles)
                .delete(handlers::guild_backup::pending_roles::clear_pending_roles),
        )
        .route(
            "/{guild_id}/pending-roles/{user_id}/consume",
            post(handlers::guild_backup::pending_roles::consume_pending_roles),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/guild-backup", inner())
}
