//! Routes audit logs + watched users + discord roles.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // Audit logs
        .route(
            "/api/audit-logs",
            get(handlers::audit::audit_logs::list_audit_logs)
                .post(handlers::audit::audit_logs::create_audit_log),
        )
        .route(
            "/api/audit-logs/{guild_id}",
            delete(handlers::audit::audit_logs::purge_audit_logs),
        )
        // Detection d'anomalie de moderation (decision server-side).
        .route(
            "/api/moderation-anomaly",
            post(handlers::audit::moderation_anomaly::detect_moderation_anomaly),
        )
        // Rapport hebdomadaire agrege server-side (fenetre 7 jours).
        .route(
            "/api/audit-weekly-report/{guild_id}",
            get(handlers::audit::weekly_report::get_weekly_report),
        )
        // Watched users
        .route(
            "/api/watched-users",
            get(handlers::audit::watched_users::list_watched_users)
                .post(handlers::audit::watched_users::add_watched_user),
        )
        .route(
            "/api/watched-users/{guild_id}/{user_id}",
            get(handlers::audit::watched_users::get_user_dossier)
                .delete(handlers::audit::watched_users::remove_watched_user),
        )
        // Discord roles (CRUD)
        .route(
            "/api/discord-roles/{guild_id}",
            get(handlers::community::discord_roles::list_roles),
        )
        .route(
            "/api/discord-roles/{guild_id}/create",
            post(handlers::community::discord_roles::create_role),
        )
        .route(
            "/api/discord-roles/{guild_id}/{role_id}",
            patch(handlers::community::discord_roles::edit_role)
                .delete(handlers::community::discord_roles::delete_role),
        )
}
