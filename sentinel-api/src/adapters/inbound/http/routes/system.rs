//! Routes systeme (user activity, models status, cache, system info, welcome, jobs async, RBAC).

use axum::routing::delete;
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
        .route(
            "/api/docker/overview",
            get(handlers::system::docker::get_overview),
        )
        .route(
            "/api/docker/containers",
            get(handlers::system::docker::list_containers),
        )
        .route(
            "/api/docker/containers/{id}",
            delete(handlers::system::docker::remove_container),
        )
        .route(
            "/api/docker/containers/{id}/start",
            post(handlers::system::docker::start_container),
        )
        .route(
            "/api/docker/containers/{id}/stop",
            post(handlers::system::docker::stop_container),
        )
        .route(
            "/api/docker/containers/{id}/restart",
            post(handlers::system::docker::restart_container),
        )
        .route(
            "/api/docker/containers/{id}/logs",
            get(handlers::system::docker::container_logs),
        )
        .route(
            "/api/docker/images",
            get(handlers::system::docker::list_images),
        )
        .route(
            "/api/docker/images/{id}",
            delete(handlers::system::docker::remove_image),
        )
        .route(
            "/api/docker/volumes",
            get(handlers::system::docker::list_volumes),
        )
        .route(
            "/api/docker/volumes/{name}",
            delete(handlers::system::docker::remove_volume),
        )
        .route(
            "/api/docker/networks",
            get(handlers::system::docker::list_networks),
        )
        .route(
            "/api/docker/prune/containers",
            post(handlers::system::docker::prune_containers),
        )
        .route(
            "/api/docker/prune/images",
            post(handlers::system::docker::prune_images),
        )
        .route(
            "/api/docker/prune/volumes",
            post(handlers::system::docker::prune_volumes),
        )
        .route(
            "/api/docker/prune/networks",
            post(handlers::system::docker::prune_networks),
        )
        .route(
            "/api/docker/prune/system",
            post(handlers::system::docker::prune_system),
        )
        .route(
            "/api/docker/prune/build-cache",
            post(handlers::system::docker::prune_build_cache),
        )
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
        // Security monitoring (admin+) : top IPs, auth failures, audit logs, TLS
        .route(
            "/api/security/top-ips",
            get(handlers::system::security::top_ips),
        )
        .route(
            "/api/security/auth-failures",
            get(handlers::system::security::auth_failures),
        )
        .route(
            "/api/security/banned-ips",
            get(handlers::system::security::banned_ips),
        )
        .route(
            "/api/security/audit-logs",
            get(handlers::system::security::audit_logs),
        )
        .route(
            "/api/security/tls-cert",
            get(handlers::system::security::tls_cert),
        )
        .route(
            "/api/security/traffic-trend",
            get(handlers::system::security::traffic_trend),
        )
        .route(
            "/api/security/last-logins",
            get(handlers::system::security::last_successful_logins),
        )
        .route(
            "/api/security/ssh-failures",
            get(handlers::system::security::ssh_failures),
        )
        .route(
            "/api/security/disk-trend",
            get(handlers::system::security::disk_trend),
        )
        .route(
            "/api/security/connections",
            get(handlers::system::security::active_connections),
        )
        .route(
            "/api/security/open-ports",
            get(handlers::system::security::open_ports),
        )
        .route(
            "/api/security/trivy",
            get(handlers::system::security::trivy_vulns),
        )
        .route(
            "/api/security/file-integrity",
            get(handlers::system::security::file_integrity),
        )
        .route(
            "/api/security/outbound",
            get(handlers::system::security::outbound_connections),
        )
        .route(
            "/api/security/nginx-suspicious",
            get(handlers::system::security::nginx_suspicious),
        )
        .route(
            "/api/security/tls-errors",
            get(handlers::system::security::tls_errors),
        )
        .route(
            "/api/security/geoip",
            get(handlers::system::security::geoip_lookup),
        )
        .route(
            "/api/security/container-changes",
            get(handlers::system::security::container_changes),
        )
        .route(
            "/api/security/cleanup",
            delete(handlers::system::security::cleanup_security_logs),
        )
        .route(
            "/api/security/server-events",
            get(handlers::system::server_events::list_server_events),
        )
        .route(
            "/api/security/ban-ip",
            post(handlers::system::security::ban_ip),
        )
        .route(
            "/api/security/unban-ip",
            post(handlers::system::security::unban_ip),
        )
        .route(
            "/api/security/manual-bans",
            get(handlers::system::security::manual_bans),
        )
}
