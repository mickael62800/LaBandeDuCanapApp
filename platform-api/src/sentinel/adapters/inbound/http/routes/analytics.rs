//! Routes analytics (inference lourde, rate limit strict).
//! Montees sous `/api/analytics`.

use axum::routing::{get, post};
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn inner() -> Router<AppState> {
    Router::new()
        // Agregat unique consomme par le front (DashboardChartsSection). Les
        // anciens sous-endpoints /heatmap /actions /top-infractors
        // /moderation-trend /peak-hours ont ete supprimes : redondants (memes
        // donnees) et sans aucun consommateur (web/bot/worker).
        .route("/", get(handlers::audit::analytics::get_full_analytics))
        .route("/reset", post(handlers::audit::analytics::reset_analytics))
        // Jobs declenches par sentinel-worker.
        .route(
            "/snapshot/daily",
            post(handlers::audit::snapshots::snapshot_daily_all),
        )
        .route(
            "/snapshot/hourly",
            post(handlers::audit::snapshots::snapshot_hourly_all),
        )
        .route(
            "/retention-cleanup",
            post(handlers::audit::snapshots::retention_cleanup_all),
        )
        .route(
            "/publish-top-users",
            post(handlers::audit::snapshots::publish_top_users_all),
        )
        .route(
            "/publish-monthly-ranking",
            post(handlers::community::monthly_ranking::publish_monthly_ranking_all),
        )
        .route(
            "/force-monthly-ranking",
            post(handlers::community::monthly_ranking::force_publish_monthly_ranking),
        )
        // Export user-facing.
        .route("/export", get(handlers::audit::snapshots::export_analytics))
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/analytics", inner())
}
