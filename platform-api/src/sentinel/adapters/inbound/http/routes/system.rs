//! Routes systeme (user activity, models status, cache, system info, welcome, jobs async, RBAC).

use axum::routing::get;
use axum::routing::post;
use axum::Router;

use super::super::handlers;
use super::super::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // User activity (surveillance)
        .route(
            "/api/user-activity",
            post(handlers::audit::user_activity::create_activity),
        )
        .route(
            "/api/user-activity/{guild_id}/by-message/{message_id}",
            get(handlers::audit::user_activity::get_by_message_id),
        )
        .route(
            "/api/user-activity/{guild_id}/{user_id}",
            get(handlers::audit::user_activity::get_activity),
        )
        // Models status (IA)
        .route(
            "/api/models/status",
            get(handlers::system::models_status::get_models_status),
        )
        .route(
            "/api/models/reload",
            post(handlers::system::models_status::reload_model),
        )
        // Cache monitoring
        .route(
            "/api/cache/stats",
            get(handlers::system::cache_stats::get_cache_stats),
        )
        // Detail systeme (bots/workers list + CPU/RAM host + uptime + taille BDD)
        .route(
            "/api/system/info",
            get(handlers::system::info::get_system_info),
        )
        // Les regles d'alerte de supervision ont demenage dans `ops-api`
        // (/ops-api/alert-rules) : elles pilotent le dispatcher d'alertes de la
        // MACHINE, pas la moderation Discord.
        // DANGER — factory reset d'un serveur (owner-only + confirmation forte)
        .route(
            "/api/system/guild-reset/{guild_id}",
            post(handlers::system::guild_reset::reset_guild),
        )
        // Welcome config
        .route(
            "/api/welcome/{guild_id}",
            get(handlers::community::welcome::get_config)
                .put(handlers::community::welcome::save_config),
        )
        .route(
            "/api/welcome/{guild_id}/rules/publish",
            post(handlers::community::welcome::publish_rules),
        )
        // Verification d'age : DECISION server-side (seuil pass/ban + duree).
        .route(
            "/api/welcome/{guild_id}/age-check",
            post(handlers::community::welcome::age_check),
        )
        // Phase 4 A — File d'attente IA async (POST = enqueue, GET = statut)
        .route("/api/ai/jobs", post(handlers::ai::ai_jobs::create_ai_job))
        .route("/api/ai/jobs/{id}", get(handlers::ai::ai_jobs::get_ai_job))
        // Phase 6 A — File d'attente exports async (infractions/audit_logs/moderation_actions, CSV/JSON)
        .route(
            "/api/exports/jobs",
            post(handlers::system::exports::create_export_job),
        )
        .route(
            "/api/exports/jobs/{id}",
            get(handlers::system::exports::get_export_job),
        )
        // Phase 1 sync Discord <-> Web : mapping action_id <-> Discord message
        .route(
            "/api/discord-messages/register",
            post(handlers::audit::discord_action_messages::register),
        )
        .route(
            "/api/discord-messages/{action_id}",
            get(handlers::audit::discord_action_messages::list_for_action),
        )
        // Phase Docker — administration via /var/run/docker.sock (gate superadmin sur les actions)
        // AI dataset (collecte messages -> CSV pour entrainement)
        .route(
            "/api/ai-dataset/messages/{guild_id}",
            get(handlers::ai::dataset::list_messages).delete(handlers::ai::dataset::bulk_delete),
        )
        // Note : la collecte des messages (ex POST /api/ai-dataset/collect) est
        // passee en gRPC `AiDatasetService.CollectMessage` (cf. audit transport).
        // Sondes d'autorisation : 200 si la requete a franchi le gate
        // superadmin, 403 sinon (le middleware repond avant le handler).
        .route(
            "/api/auth/check-access",
            get(handlers::system::nexus_access::check_access),
        )
        // Cible de `auth_request` nginx pour la passerelle /nexus-api/.
        .route(
            "/api/auth/nexus-access",
            get(handlers::system::nexus_access::nexus_access),
        )
        .route(
            "/api/internal/jobs/{job}",
            post(handlers::system::internal_jobs::run),
        )
    // Security monitoring (admin+) : top IPs, auth failures, audit logs, TLS
}
