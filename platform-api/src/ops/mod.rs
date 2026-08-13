//! API d'exploitation de la machine hote.
//!
//! # Perimetre
//!
//! Ce qui parle de la MACHINE, pas de Discord : sondes systeme, conteneurs
//! Docker, logs techniques des services, securite de l'hote (TLS, IP bannies,
//! journal d'administration), regles d'alerte. Cette machine heberge Sentinel,
//! Nexus et Atrium : ces ecrans leur sont transverses, ils n'appartiennent a
//! aucune des trois.
//!
//! # Acces
//!
//! Meme montage que `nexus-api` et `atrium-api` : le navigateur passe par la
//! passerelle nginx `/ops-api/`, qui valide la session Discord (`auth_request`
//! vers sentinel-api, seul composant a savoir QUI est connecte) puis injecte
//! `OPS_API_TOKEN` cote serveur. Le jeton ne parvient jamais au navigateur.
//!
//! # Base de donnees
//!
//! Ce service se connecte a la base de Sentinel (`discord_sentinel`) — et non a
//! une base dediee. Postgres ne sait pas requeter entre bases logiques, or
//! l'exploitation doit LIRE `logs` et `audit_logs`, ecrites par Sentinel et ses
//! bots sur le chemin chaud. Il partage donc le role applicatif `sentinel_app`
//! avec `sentinel-api` et `sentinel-worker`, et lit les logs via les vues
//! `ops_logs_v` / `ops_audit_logs_v`.
//!
//! Un role Postgres restreint par service (`sentinel_ops`, migration 024) avait
//! ete tente puis ABANDONNE en migration 028 : jamais utilise, droits
//! incomplets, il donnait l'illusion d'un cloisonnement inexistant. Ne pas le
//! reintroduire (cf. CLAUDE.md).

use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use platform_common_api::{rate_limit_middleware, RateLimiter};
use platform_core::ops::ports::inbound::manage_alert_rules::ManageAlertRulesUseCase;

pub mod adapters;
pub mod config;
pub mod container_monitor;
pub mod handlers;
pub mod jobs;

pub use config::AppConfig;

/// Etat partage. Un seul domaine ici : pas de sous-etats, contrairement a
/// `sentinel-api` — c'est justement le signe que le perimetre est etroit.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub alert_rules_uc: Arc<dyn ManageAlertRulesUseCase>,
    /// Daemon Docker de l'hote, via `docker-agent`. Ce processus ne monte
    /// jamais le socket : il passe par l'agent, seul a le porter.
    pub docker_host: Arc<dyn platform_core::ops::ports::outbound::docker_host::DockerHost>,
    /// Journal des evenements de la machine (qui a arrete quoi).
    pub server_events: Arc<
        dyn platform_core::ops::ports::outbound::server_event_repository::ServerEventRepository,
    >,
    // -- Logs systeme --
    pub system_logs_uc:
        Arc<dyn platform_core::ops::ports::inbound::manage_system_logs::ManageSystemLogsUseCase>,
    /// Connexion Redis multiplexee et auto-reconnectante, partagee par toutes
    /// les requetes (streams de logs, snapshot conteneurs, sonde readiness). Un
    /// clone partage le meme pipe : plus d'ouverture de connexion par appel.
    pub redis_client: redis::aio::ConnectionManager,

    // ── Securite de l'hote ──
    pub security_logs_uc:
        Arc<dyn platform_core::ops::ports::inbound::read_security_logs::ReadSecurityLogsUseCase>,
    pub security_audit_uc: Arc<
        dyn platform_core::ops::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase,
    >,
    pub host_probe_uc:
        Arc<dyn platform_core::ops::ports::inbound::read_host_probe::ReadHostProbeUseCase>,
    pub tls_cert_uc: Arc<dyn platform_core::ops::ports::inbound::read_tls_cert::ReadTlsCertUseCase>,
    pub ip_bans_uc:
        Arc<dyn platform_core::ops::ports::inbound::manage_ip_bans::ManageIpBansUseCase>,
    pub geoip_uc: Arc<dyn platform_core::ops::ports::inbound::lookup_geoip::LookupGeoIpUseCase>,
    pub server_events_uc: Arc<
        dyn platform_core::ops::ports::inbound::manage_server_events::ManageServerEventsUseCase,
    >,
    /// Pool Postgres brut, uniquement pour la sonde de readiness (`SELECT 1`).
    /// Le metier passe par les ports ; ceci n'est qu'un ping d'infrastructure.
    pub pg_pool: sqlx::PgPool,
}

impl FromRef<AppState> for Arc<AppConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

/// Erreur HTTP de l'API.
pub struct ApiError(pub StatusCode, pub String);

impl ApiError {
    pub fn not_found(what: &str) -> Self {
        Self(StatusCode::NOT_FOUND, what.to_owned())
    }
}

impl From<platform_core::ops::domain::errors::DomainError> for ApiError {
    fn from(error: platform_core::ops::domain::errors::DomainError) -> Self {
        use platform_core::ops::domain::errors::DomainError as E;
        let status = match &error {
            E::NotFound(_) => StatusCode::NOT_FOUND,
            E::ValidationError(_) | E::Validation(_) => StatusCode::BAD_REQUEST,
            E::Conflict(_) => StatusCode::CONFLICT,
            E::Forbidden(_) => StatusCode::FORBIDDEN,
            E::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
            E::Timeout(_) => StatusCode::GATEWAY_TIMEOUT,
            E::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            E::Internal(_) | E::Infrastructure(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self(
            status,
            platform_common_api::errors::public_message(status, &error),
        )
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        platform_common_api::errors::error_response(self.0, &self.1)
    }
}

pub fn router(state: AppState) -> Router {
    let rate_limiter = RateLimiter::new(state.config.rate_limit_per_sec);
    let bearer =
        platform_common_api::bearer_auth::RequiredBearerToken::new(state.config.api_token.clone())
            .with_scheduler(std::env::var("OPS_SCHEDULER_TOKEN").unwrap_or_default());

    let protected = Router::new()
        .route(
            "/internal/jobs/dispatch-alerts",
            axum::routing::post(handlers::internal_jobs::dispatch_alerts),
        )
        .route("/alert-rules", get(handlers::alert_rules::list))
        // ── Conteneurs de l'hote ──
        .route(
            "/docker/overview",
            get(handlers::docker::overview::get_overview),
        )
        .route(
            "/docker/containers",
            get(handlers::docker::containers::list_containers),
        )
        .route(
            "/docker/containers/{id}/start",
            axum::routing::post(handlers::docker::containers::start_container),
        )
        .route(
            "/docker/containers/{id}/stop",
            axum::routing::post(handlers::docker::containers::stop_container),
        )
        .route(
            "/docker/containers/{id}/restart",
            axum::routing::post(handlers::docker::containers::restart_container),
        )
        .route(
            "/docker/containers/{id}",
            axum::routing::delete(handlers::docker::containers::remove_container),
        )
        .route(
            "/docker/containers/{id}/logs",
            get(handlers::docker::containers::container_logs),
        )
        .route("/docker/images", get(handlers::docker::images::list_images))
        .route(
            "/docker/images/{id}",
            axum::routing::delete(handlers::docker::images::remove_image),
        )
        .route(
            "/docker/volumes",
            get(handlers::docker::volumes::list_volumes),
        )
        .route(
            "/docker/volumes/{name}",
            axum::routing::delete(handlers::docker::volumes::remove_volume),
        )
        .route(
            "/docker/networks",
            get(handlers::docker::networks::list_networks),
        )
        .route(
            "/docker/prune/containers",
            axum::routing::post(handlers::docker::prune::prune_containers),
        )
        .route(
            "/docker/prune/images",
            axum::routing::post(handlers::docker::prune::prune_images),
        )
        .route(
            "/docker/prune/volumes",
            axum::routing::post(handlers::docker::prune::prune_volumes),
        )
        .route(
            "/docker/prune/networks",
            axum::routing::post(handlers::docker::prune::prune_networks),
        )
        .route(
            "/docker/prune/system",
            axum::routing::post(handlers::docker::prune::prune_system),
        )
        .route(
            "/docker/prune/build-cache",
            axum::routing::post(handlers::docker::prune::prune_build_cache),
        )
        .route(
            "/containers/changes",
            get(handlers::docker::changes::container_changes),
        )
        // ── Securite de l'hote ──
        .route(
            "/security/server-events",
            get(handlers::server_events::list_server_events),
        )
        .route("/security/top-ips", get(handlers::security::logs::top_ips))
        .route(
            "/security/auth-failures",
            get(handlers::security::logs::auth_failures),
        )
        .route(
            "/security/banned-ips",
            get(handlers::security::bans::banned_ips),
        )
        .route(
            "/security/manual-bans",
            get(handlers::security::bans::manual_bans),
        )
        .route(
            "/security/ban-ip",
            axum::routing::post(handlers::security::bans::ban_ip),
        )
        .route(
            "/security/unban-ip",
            axum::routing::post(handlers::security::bans::unban_ip),
        )
        .route(
            "/security/ssh-failures",
            get(handlers::security::probes::ssh_failures),
        )
        .route(
            "/security/open-ports",
            get(handlers::security::probes::open_ports),
        )
        .route(
            "/security/file-integrity",
            get(handlers::security::probes::file_integrity),
        )
        .route(
            "/security/trivy",
            get(handlers::security::probes::trivy_vulns),
        )
        .route(
            "/security/disk-trend",
            get(handlers::security::probes::disk_trend),
        )
        .route(
            "/security/connections",
            get(handlers::security::probes::active_connections),
        )
        .route(
            "/security/outbound",
            get(handlers::security::probes::outbound_connections),
        )
        .route(
            "/security/nginx-suspicious",
            get(handlers::security::probes::nginx_suspicious),
        )
        .route("/security/tls-cert", get(handlers::security::tls::tls_cert))
        .route(
            "/security/tls-errors",
            get(handlers::security::tls::tls_errors),
        )
        .route(
            "/security/traffic-trend",
            get(handlers::security::logs::traffic_trend),
        )
        .route(
            "/security/geoip",
            get(handlers::security::geoip::geoip_lookup),
        )
        .route(
            "/security/audit-logs",
            get(handlers::security::audit::audit_logs),
        )
        .route(
            "/security/last-logins",
            get(handlers::security::audit::last_successful_logins),
        )
        .route(
            "/security/cleanup",
            axum::routing::delete(handlers::security::audit::cleanup_security_logs),
        )
        .route(
            "/alert-rules/{id}",
            axum::routing::patch(handlers::alert_rules::update),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            bearer,
            platform_common_api::bearer_auth::require,
        ));

    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/metrics", get(handlers::metrics::metrics))
        .merge(protected)
        .layer(axum::middleware::from_fn_with_state(
            state.pg_pool.clone(),
            platform_common_api::job_lock::middleware,
        ))
        .with_state(state)
        .layer(axum::middleware::from_fn(
            platform_common_api::metrics::metrics_middleware,
        ))
        .route_layer(axum::middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit_middleware,
        ));

    platform_common_api::http::security_headers(router).with_state(rate_limiter)
}

/// Liveness : le processus tourne. Volontairement sans dependance externe — une
/// panne de Postgres ou Redis ne doit PAS declencher un redemarrage du conteneur
/// (ca n'y changerait rien et couperait le back-office pendant l'incident).
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

/// Readiness : l'API peut-elle reellement servir ? Verifie Postgres et Redis EN
/// PARALLELE. Le Docker Agent est une dependance DEGRADEE : son indisponibilite
/// n'empeche pas de servir logs, securite et audit, donc elle n'echoue pas la
/// readiness — elle est seulement rapportee.
async fn ready(State(state): State<AppState>) -> axum::response::Response {
    let (postgres, redis, docker) = tokio::join!(
        check_postgres(&state.pg_pool),
        check_redis(&state.redis_client),
        check_docker(&state.docker_host),
    );

    // Seules Postgres et Redis conditionnent la readiness.
    let ready = postgres && redis;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let body = Json(serde_json::json!({
        "status": if ready { "ready" } else { "not_ready" },
        "postgres": postgres,
        "redis": redis,
        "docker_agent": docker, // dependance degradee, informative
    }));
    (status, body).into_response()
}

async fn check_postgres(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

async fn check_redis(manager: &redis::aio::ConnectionManager) -> bool {
    let mut conn = manager.clone();
    redis::cmd("PING")
        .query_async::<String>(&mut conn)
        .await
        .is_ok()
}

async fn check_docker(
    docker: &Arc<dyn platform_core::ops::ports::outbound::docker_host::DockerHost>,
) -> bool {
    docker.version_info().await.is_ok()
}
