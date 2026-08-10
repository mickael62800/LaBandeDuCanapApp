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
//! Ce service se connecte a la base de Sentinel — et non a une base dediee.
//! Postgres ne sait pas requeter entre bases logiques, or l'exploitation doit
//! LIRE `logs` et `audit_logs`, qui sont ecrites par Sentinel et ses bots sur
//! le chemin chaud. La separation passe donc par un ROLE Postgres restreint :
//! lecture-ecriture sur les tables que l'exploitation possede (`alert_rules`,
//! `ip_bans`, `server_events`), lecture seule sur les vues `ops_logs_v` et
//! `ops_audit_logs_v`. La regle de propriete est ainsi verifiee par la base,
//! pas tenue par convention.

use std::sync::Arc;

use axum::extract::FromRef;
use axum::http::{header::AUTHORIZATION, HeaderMap, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use ops_core::ports::inbound::manage_alert_rules::ManageAlertRulesUseCase;
use platform_common_api::{rate_limit_middleware, RateLimiter};

pub mod adapters;
pub mod alerts_dispatcher;
pub mod container_monitor;
pub mod config;
pub mod handlers;

pub use config::AppConfig;

/// Etat partage. Un seul domaine ici : pas de sous-etats, contrairement a
/// `sentinel-api` — c'est justement le signe que le perimetre est etroit.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub alert_rules_uc: Arc<dyn ManageAlertRulesUseCase>,
    /// Daemon Docker de l'hote, via `docker-agent`. Ce processus ne monte
    /// jamais le socket : il passe par l'agent, seul a le porter.
    pub docker_host: Arc<dyn ops_core::ports::outbound::docker_host::DockerHost>,
    /// Journal des evenements de la machine (qui a arrete quoi).
    pub server_events: Arc<dyn ops_core::ports::outbound::server_event_repository::ServerEventRepository>,
    /// Instantane des conteneurs, alimente par `container_monitor`. Partage en
    /// memoire avec la boucle : le processus qui produit la donnee est celui
    /// qui la sert.
    pub container_monitor: container_monitor::SharedMonitorState,

    // -- Logs systeme --
    pub system_logs_uc: Arc<dyn ops_core::ports::inbound::manage_system_logs::ManageSystemLogsUseCase>,
    pub redis_client: redis::Client,

    // ── Securite de l'hote ──
    pub security_logs_uc: Arc<dyn ops_core::ports::inbound::read_security_logs::ReadSecurityLogsUseCase>,
    pub security_audit_uc: Arc<dyn ops_core::ports::inbound::manage_security_audit::ManageSecurityAuditUseCase>,
    pub host_probe_uc: Arc<dyn ops_core::ports::inbound::read_host_probe::ReadHostProbeUseCase>,
    pub tls_cert_uc: Arc<dyn ops_core::ports::inbound::read_tls_cert::ReadTlsCertUseCase>,
    pub ip_bans_uc: Arc<dyn ops_core::ports::inbound::manage_ip_bans::ManageIpBansUseCase>,
    pub geoip_uc: Arc<dyn ops_core::ports::inbound::lookup_geoip::LookupGeoIpUseCase>,
    pub server_events_uc: Arc<dyn ops_core::ports::inbound::manage_server_events::ManageServerEventsUseCase>,
}

impl FromRef<AppState> for Arc<AppConfig> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

/// Erreur HTTP de l'API.
pub struct ApiError(pub StatusCode, pub String);

impl ApiError {
    pub fn unauthorized() -> Self {
        Self(StatusCode::UNAUTHORIZED, "jeton API invalide".into())
    }
    pub fn not_found(what: &str) -> Self {
        Self(StatusCode::NOT_FOUND, what.to_owned())
    }
}

impl From<ops_core::domain::errors::DomainError> for ApiError {
    fn from(error: ops_core::domain::errors::DomainError) -> Self {
        use ops_core::domain::errors::DomainError as E;
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
        // Le detail technique part dans les logs, pas dans la reponse : cette
        // API n'est appelee que par le back-office, et une trace SQL dans le
        // navigateur ne rend service qu'a un attaquant.
        if status.is_server_error() {
            tracing::error!(%error, "erreur interne");
            return Self(status, "erreur interne".into());
        }
        Self(status, error.to_string())
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.0, Json(serde_json::json!({ "error": self.1 }))).into_response()
    }
}

/// Verifie le jeton injecte par nginx. L'API n'a aucune notion d'utilisateur :
/// l'identite a deja ete resolue par la passerelle.
pub fn authorize(headers: &HeaderMap, config: &AppConfig) -> Result<(), ApiError> {
    let expected = format!("Bearer {}", config.api_token);
    let supplied = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok());
    if supplied == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(ApiError::unauthorized())
    }
}

pub fn router(state: AppState) -> Router {
    let rate_limiter = RateLimiter::new(state.config.rate_limit_per_sec);

    let router = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(handlers::metrics::metrics))
        .route("/alert-rules", get(handlers::alert_rules::list))
        // ── Conteneurs de l'hote ──
        .route("/docker/overview", get(handlers::docker::get_overview))
        .route("/docker/containers", get(handlers::docker::list_containers))
        .route("/docker/containers/{id}/start", axum::routing::post(handlers::docker::start_container))
        .route("/docker/containers/{id}/stop", axum::routing::post(handlers::docker::stop_container))
        .route("/docker/containers/{id}/restart", axum::routing::post(handlers::docker::restart_container))
        .route("/docker/containers/{id}", axum::routing::delete(handlers::docker::remove_container))
        .route("/docker/containers/{id}/logs", get(handlers::docker::container_logs))
        .route("/docker/images", get(handlers::docker::list_images))
        .route("/docker/images/{id}", axum::routing::delete(handlers::docker::remove_image))
        .route("/docker/volumes", get(handlers::docker::list_volumes))
        .route("/docker/volumes/{name}", axum::routing::delete(handlers::docker::remove_volume))
        .route("/docker/networks", get(handlers::docker::list_networks))
        .route("/docker/prune/containers", axum::routing::post(handlers::docker::prune_containers))
        .route("/docker/prune/images", axum::routing::post(handlers::docker::prune_images))
        .route("/docker/prune/volumes", axum::routing::post(handlers::docker::prune_volumes))
        .route("/docker/prune/networks", axum::routing::post(handlers::docker::prune_networks))
        .route("/docker/prune/system", axum::routing::post(handlers::docker::prune_system))
        .route("/docker/prune/build-cache", axum::routing::post(handlers::docker::prune_build_cache))
        .route("/containers/changes", get(handlers::docker::container_changes))
        // ── Securite de l'hote ──
        .route("/security/server-events", get(handlers::server_events::list_server_events))
        .route("/security/top-ips", get(handlers::security::top_ips))
        .route("/security/auth-failures", get(handlers::security::auth_failures))
        .route("/security/banned-ips", get(handlers::security::banned_ips))
        .route("/security/manual-bans", get(handlers::security::manual_bans))
        .route("/security/ban-ip", axum::routing::post(handlers::security::ban_ip))
        .route("/security/unban-ip", axum::routing::post(handlers::security::unban_ip))
        .route("/security/ssh-failures", get(handlers::security::ssh_failures))
        .route("/security/open-ports", get(handlers::security::open_ports))
        .route("/security/file-integrity", get(handlers::security::file_integrity))
        .route("/security/trivy", get(handlers::security::trivy_vulns))
        .route("/security/disk-trend", get(handlers::security::disk_trend))
        .route("/security/connections", get(handlers::security::active_connections))
        .route("/security/outbound", get(handlers::security::outbound_connections))
        .route("/security/nginx-suspicious", get(handlers::security::nginx_suspicious))
        .route("/security/tls-cert", get(handlers::security::tls_cert))
        .route("/security/tls-errors", get(handlers::security::tls_errors))
        .route("/security/traffic-trend", get(handlers::security::traffic_trend))
        .route("/security/geoip", get(handlers::security::geoip_lookup))
        .route("/security/audit-logs", get(handlers::security::audit_logs))
        .route("/security/last-logins", get(handlers::security::last_successful_logins))
        .route("/security/cleanup", axum::routing::delete(handlers::security::cleanup_security_logs))
        .route(
            "/alert-rules/{id}",
            axum::routing::patch(handlers::alert_rules::update),
        )
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

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
