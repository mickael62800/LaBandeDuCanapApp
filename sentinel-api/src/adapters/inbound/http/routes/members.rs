//! Routes members (DB-backed + liste Discord directe).
//!
//! Restaurees apres un nettoyage trop large : les handlers passent par
//! `CommunityState.members_uc` (metier survivant). Les routes salons/emojis,
//! elles, vivent desormais dans `routes/guilds.rs` — ne pas les redeclarer ici
//! sous peine de doublon de route (panique axum au demarrage).

use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn member_inner() -> Router<AppState> {
    Router::new()
        .route(
            "/{guild_id}",
            get(handlers::community::guild_members::list_members_db),
        )
        .route(
            "/{guild_id}/{user_id}",
            get(handlers::community::guild_members::get_member)
                .patch(handlers::community::guild_members::update_member)
                .delete(handlers::community::guild_members::remove_member),
        )
        .route(
            "/{guild_id}/{user_id}/summary",
            get(handlers::community::guild_members::get_member_summary),
        )
        .route(
            "/{guild_id}/{user_id}/reset",
            post(handlers::community::guild_members::reset_member),
        )
        // Lifecycle : appeles par sentinel-bot sur GuildMemberRemove/Add.
        .route(
            "/{guild_id}/{user_id}/leave",
            post(handlers::community::guild_members::leave_member),
        )
        .route(
            "/{guild_id}/{user_id}/rejoin",
            post(handlers::community::guild_members::rejoin_member),
        )
        .route(
            "/sync",
            post(handlers::community::guild_members::sync_members),
        )
        .route(
            "/register",
            post(handlers::community::guild_members::register_member),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Membres (DB-backed)
        .nest("/api/members", member_inner())
        // Liste des membres Discord (cache Redis + fallback API Discord)
        .route(
            "/api/guilds/{guild_id}/members",
            get(handlers::community::guild_members::list_members),
        )
}
