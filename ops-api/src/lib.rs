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
pub mod config;
pub mod handlers;

pub use config::AppConfig;

/// Etat partage. Un seul domaine ici : pas de sous-etats, contrairement a
/// `sentinel-api` — c'est justement le signe que le perimetre est etroit.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub alert_rules_uc: Arc<dyn ManageAlertRulesUseCase>,
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
