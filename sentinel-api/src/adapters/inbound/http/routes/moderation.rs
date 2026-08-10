//! Routes moderation + strikes + notes + reminders.

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn moderation_inner() -> Router<AppState> {
    Router::new()
        .route("/actions", post(handlers::moderation::actions::log_action))
        .route(
            "/actions/{id}",
            delete(handlers::moderation::actions::delete_action),
        )
        // Quota par moderateur : nombre d'actions sur une fenetre (garde-fou bot).
        .route(
            "/mod-action-count/{guild_id}/{moderator_id}",
            get(handlers::moderation::actions::mod_action_count),
        )
        // Ban en sursis
        .route(
            "/{guild_id}/sursis",
            post(handlers::moderation::sursis::create_sursis),
        )
        .route(
            "/sursis/{id}",
            get(handlers::moderation::sursis::get_sursis),
        )
        .route(
            "/sursis/{id}/resolve",
            post(handlers::moderation::sursis::resolve_sursis),
        )
        .route(
            "/internal/jobs/sursis-expire",
            post(handlers::moderation::sursis::job_sursis_expire),
        )
        .route("/bans", get(handlers::moderation::actions::list_bans))
        .route(
            "/execute-ban",
            post(handlers::moderation::actions::execute_ban),
        )
        .route(
            "/execute-unban",
            post(handlers::moderation::actions::execute_unban),
        )
        .route(
            "/execute-mute",
            post(handlers::moderation::actions::execute_mute),
        )
        .route(
            "/history/{guild_id}/{user_id}",
            get(handlers::moderation::actions::get_history),
        )
                .route(
            "/{guild_id}/assess-target-risk",
            post(handlers::moderation::target_risk::assess_target_risk),
        )
        .route(
            "/modstats/{guild_id}",
            get(handlers::moderation::actions::get_modstats),
        )
        .route(
            "/modstats/{guild_id}/trend",
            get(handlers::moderation::actions::get_modstats_trend),
        )
        .route(
            "/evidence",
            post(handlers::moderation::actions::add_evidence),
        )
        .route(
            "/evidence/{action_id}",
            get(handlers::moderation::actions::list_evidence),
        )
        .route("/review", post(handlers::moderation::actions::add_review))
        .route(
            "/review/{guild_id}/pending",
            get(handlers::moderation::actions::list_pending_reviews),
        )
        .route(
            "/review/{id}/resolve",
            patch(handlers::moderation::actions::resolve_review),
        )
}

/// Strikes (avertissements a paliers), montes sous `/api/strikes`.
fn strikes_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/config/{guild_id}",
            get(handlers::moderation::strikes::get_config)
                .put(handlers::moderation::strikes::save_config),
        )
        .route(
            "/{guild_id}/{user_id}",
            get(handlers::moderation::strikes::get_active_strikes)
                .delete(handlers::moderation::strikes::reset_strikes),
        )
        .route("/", post(handlers::moderation::strikes::add_strike))
}

/// Notes moderateurs, montees sous `/api/notes`.
fn notes_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::moderation::notes::add_note))
        .route(
            "/{guild_id}/{user_id}",
            get(handlers::moderation::notes::get_notes),
        )
        .route("/{id}", delete(handlers::moderation::notes::delete_note))
}

/// Rappels de sanction, montes sous `/api/reminders`.
fn reminders_inner() -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::moderation::reminders::create_reminder))
        .route("/pending", get(handlers::moderation::reminders::get_pending))
        .route(
            "/{guild_id}",
            get(handlers::moderation::reminders::list_by_guild),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/api/moderation", moderation_inner())
        .nest("/api/strikes", strikes_inner())
        .nest("/api/notes", notes_inner())
        .nest("/api/reminders", reminders_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression : la route dynamique `/{guild_id}/copilot/{user_id}` coexiste
    /// avec les routes statiques (`/actions`, `/history/...`, etc.) sans
    /// conflit matchit (axum 0.8). La construction du routeur ne doit pas paniquer.
    #[test]
    fn moderation_router_builds_without_conflict() {
        let _ = moderation_inner();
        let _ = routes();
    }
}


