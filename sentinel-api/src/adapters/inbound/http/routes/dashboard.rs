//! Routes dashboard & config (guildes, logs, infractions, bots, IA, purge).

use axum::routing::delete;
use axum::routing::get;
use axum::routing::patch;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

fn dashboard_inner() -> Router<AppState> {
    Router::new()
        .route("/guilds", get(handlers::audit::dashboard::list_guilds))
        .route(
            "/guilds/register",
            post(handlers::audit::dashboard::register_guild),
        )
        .route(
            "/guilds/reconcile",
            post(handlers::audit::dashboard::reconcile_guilds),
        )
        .route(
            "/guilds/{guild_id}",
            delete(handlers::audit::dashboard::delete_guild),
        )
        .route(
            "/logs",
            get(handlers::audit::dashboard::get_logs)
                .post(handlers::audit::dashboard::create_log),
        )
        .route(
            "/infractions",
            get(handlers::audit::dashboard::get_all_infractions),
        )
        .route(
            "/infractions/{id}",
            delete(handlers::moderation::infractions::delete_infraction),
        )
        // GET liste les regles, POST en cree/met a jour une.
        //
        // Le POST existait deja mais uniquement a la RACINE (`routes::bot`),
        // ou seul le bot l'atteint : nginx ne relaie que `/api/`. Le
        // back-office postait donc sur `/rules` et recevait un 405 d'nginx,
        // qui servait le SPA au lieu de l'API. L'enregistrement d'un poids
        // n'a jamais pu fonctionner depuis le navigateur.
        .route(
            "/rules",
            get(handlers::audit::dashboard::get_all_rules)
                .post(handlers::moderation::rules::create_rule),
        )
        .route(
            "/rules/{id}",
            patch(handlers::audit::dashboard::toggle_rule),
        )
        .route(
            "/bots/heartbeat",
            post(handlers::audit::dashboard::bot_heartbeat),
        )
        .route(
            "/bots/definitions",
            get(handlers::system::bot_config::get_definitions),
        )
        .route(
            "/bots/config/{guild_id}",
            get(handlers::system::bot_config::get_guild_config),
        )
        .route(
            "/bots/config/{guild_id}/{bot_name}",
            get(handlers::system::bot_config::get_bot_config),
        )
        .route(
            "/bots/config",
            post(handlers::system::bot_config::set_config)
                .delete(handlers::system::bot_config::delete_config),
        )
        .route(
            "/purge/infractions",
            delete(handlers::moderation::purge::purge_infractions),
        )
        .route(
            "/purge/audit-logs",
            delete(handlers::moderation::purge::purge_audit_logs),
        )
        .route(
            "/purge/logs",
            delete(handlers::moderation::purge::purge_logs),
        )
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Dashboard & config routes
        .nest("/api", dashboard_inner())
        // Charts
        .route(
            "/api/charts/activity",
            get(handlers::audit::dashboard_charts::get_activity_trend),
        )
}
