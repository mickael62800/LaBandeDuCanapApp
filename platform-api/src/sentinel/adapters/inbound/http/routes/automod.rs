//! Routes automod (montees sous `/api/automod`).
//! Phase 4 — page web /automod : timeline des detections.
//! Phase Sync — review cards (sync Discord <-> web).

use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn automod_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}/detections",
            get(handlers::moderation::automod::list_detections),
        )
        .route(
            "/{guild_id}/reviews",
            get(handlers::moderation::automod::list_reviews),
        )
        .route(
            "/{guild_id}/fp-stats",
            get(handlers::moderation::automod::fp_stats),
        )
        .route(
            "/{guild_id}/reviews/by-message/{message_id}",
            get(handlers::moderation::automod::find_review_by_message),
        )
        .route(
            "/reviews",
            post(handlers::moderation::automod::create_review),
        )
        .route(
            "/cleanup-expired-cards",
            post(handlers::moderation::automod::cleanup_expired_cards),
        )
        .route(
            "/internal/jobs/close-votes",
            post(handlers::moderation::automod::job_close_expired_votes),
        )
        .route(
            "/adaptive-slowmode",
            get(handlers::moderation::automod::list_adaptive_slowmode)
                .post(handlers::moderation::automod::mark_adaptive_slowmode),
        )
        .route(
            "/adaptive-slowmode/remove",
            post(handlers::moderation::automod::unmark_adaptive_slowmode),
        )
        .route(
            "/reviews/{review_id}/resolve",
            post(handlers::moderation::automod::resolve_review),
        )
        .route(
            "/reviews/{review_id}/ignore",
            post(handlers::moderation::automod::ignore_review),
        )
        .route(
            "/reviews/{review_id}/reopen",
            post(handlers::moderation::automod::reopen_review),
        )
        .route(
            "/reviews/{review_id}",
            get(handlers::moderation::automod::get_review),
        )
        .route(
            "/reviews/{review_id}/vote",
            post(handlers::moderation::automod::vote_review),
        )
        .route(
            "/reviews/{review_id}/votes",
            get(handlers::moderation::automod::list_review_votes),
        )
        .route(
            "/reviews/{review_id}/decide",
            post(handlers::moderation::automod::decide_review),
        )
        .route(
            "/reviews/{review_id}/discussion",
            get(handlers::moderation::automod::get_discussion)
                .post(handlers::moderation::automod::open_discussion)
                .delete(handlers::moderation::automod::delete_discussion),
        )
        .route(
            "/reviews/{review_id}/discussion/messages",
            get(handlers::moderation::automod::list_discussion_messages)
                .post(handlers::moderation::automod::append_discussion_messages),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new().nest("/api/automod", automod_inner())
}
