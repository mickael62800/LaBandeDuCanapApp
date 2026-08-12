use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;

use tracing::warn;

use crate::adapters::inbound::http::state::AppState;
use crate::adapters::outbound::system::rate_limiter::RateLimiter;
use crate::adapters::outbound::system::redis_log_stream;
use ops_core::domain::entities::log_entry::LogEntry;
use ops_core::ports::outbound::log_repository::LogRepository;

#[derive(Clone)]
pub struct ApiLoggerState {
    pub log_repo: Arc<dyn LogRepository>,
    pub redis_client: redis::Client,
    pub rate_limiter: Option<Arc<RateLimiter>>,
}

impl ApiLoggerState {
    pub fn from_app(state: &AppState) -> Self {
        Self {
            log_repo: state.shared.log_repo.clone(),
            redis_client: state.shared.redis_client.clone(),
            rate_limiter: state.ops.rate_limiter.clone(),
        }
    }
}

pub async fn api_logger_middleware(
    State(s): State<ApiLoggerState>,
    request: Request,
    next: Next,
) -> Response {
    let log_repo = s.log_repo.clone();
    let redis_client = s.redis_client.clone();
    let rate_limiter = s.rate_limiter.clone();
    let method = request.method().to_string();
    let uri = request.uri().path().to_string();
    // IP client : meme resolution que le rate limiter du socle, et pour la meme
    // raison. Prendre la PREMIERE valeur de `X-Forwarded-For` — ce que faisait
    // ce middleware — revient a lire une chaine entierement fournie par le
    // client : il suffisait d'en changer a chaque requete pour ne jamais
    // accumuler de compteur, ou d'y mettre l'IP de quelqu'un d'autre pour la
    // faire bannir a sa place. `client_ip` compte les sauts depuis la DROITE,
    // ou nos propres proxies ecrivent.
    let client_ip = platform_common_api::rate_limit::client_ip(&request, peer_ip(&request))
        .to_string();

    // Rate limit dynamique : track + ban auto si seuil franchi
    if let Some(rl) = &rate_limiter {
        if rl.observe(&client_ip).await {
            let rl_clone = rl.clone();
            let ip = client_ip.clone();
            tokio::spawn(async move {
                rl_clone.trigger_ban(ip).await;
            });
        }
    }

    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(200).collect::<String>())
        .unwrap_or_default();
    let start = Instant::now();

    let response = next.run(request).await;

    let latency = start.elapsed();
    let status = response.status().as_u16();

    let skip = uri.contains("/heartbeat") || uri == "/health";

    if !skip && (status >= 400 || latency.as_secs() >= 2 || is_mutation(&method)) {
        let level = if status >= 500 {
            "error"
        } else if status >= 400 {
            "warn"
        } else {
            "info"
        };

        let status_text = status_label(status);

        let message = format!(
            "[{}] {} {} — {} {}",
            level.to_uppercase(),
            method,
            uri,
            status,
            status_text
        );

        let details = serde_json::json!({
            "method": method,
            "route": uri,
            "status_code": status,
            "status_text": status_text,
            "latency_ms": latency.as_millis() as u64,
            "client_ip": client_ip,
            "user_agent": user_agent,
        });

        let entry = LogEntry {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            level: level.to_string(),
            bot: "sentinel-api".to_string(),
            server: String::new(),
            message,
            category: "api".to_string(),
            details,
        };

        let repo = log_repo.clone();
        tokio::spawn(async move {
            // La page Logs techniques lit les streams Redis par categorie.
            // Sans ce XADD, les logs API existaient seulement dans Postgres et
            // la colonne `api` restait vide.
            redis_log_stream::xadd_log(&redis_client, &entry).await;

            // Postgres ne conserve que les niveaux utiles a la forensique,
            // comme le endpoint POST /api/logs utilise par les autres services.
            if matches!(entry.level.as_str(), "warn" | "warning" | "error" | "fatal") {
                if let Err(e) = repo.save(&entry).await {
                    warn!(error = %e, "Echec sauvegarde log API");
                }
            }
        });
    }

    response
}

/// Adresse de la socket, seule source non falsifiable. `ConnectInfo` est pose
/// par `into_make_service_with_connect_info` ; il est absent du routeur de test,
/// d'ou le repli sur l'adresse non specifiee — que `RateLimiter::observe`
/// ignore, plutot que de compter toutes les requetes de test sur un meme bucket.
fn peer_ip(request: &Request) -> IpAddr {
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip())
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

fn is_mutation(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
}

fn status_label(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_mutation_detects_write_methods() {
        assert!(is_mutation("POST"));
        assert!(is_mutation("PUT"));
        assert!(is_mutation("PATCH"));
        assert!(is_mutation("DELETE"));
    }

    #[test]
    fn is_mutation_rejects_read_methods() {
        assert!(!is_mutation("GET"));
        assert!(!is_mutation("HEAD"));
        assert!(!is_mutation("OPTIONS"));
        assert!(!is_mutation("post")); // case-sensitive
    }

    #[test]
    fn status_label_known_codes() {
        assert_eq!(status_label(200), "OK");
        assert_eq!(status_label(201), "Created");
        assert_eq!(status_label(204), "No Content");
        assert_eq!(status_label(400), "Bad Request");
        assert_eq!(status_label(401), "Unauthorized");
        assert_eq!(status_label(403), "Forbidden");
        assert_eq!(status_label(404), "Not Found");
        assert_eq!(status_label(422), "Unprocessable Entity");
        assert_eq!(status_label(429), "Too Many Requests");
        assert_eq!(status_label(500), "Internal Server Error");
        assert_eq!(status_label(502), "Bad Gateway");
        assert_eq!(status_label(503), "Service Unavailable");
    }

    #[test]
    fn status_label_unknown_returns_unknown() {
        assert_eq!(status_label(100), "Unknown");
        assert_eq!(status_label(302), "Unknown");
        assert_eq!(status_label(418), "Unknown");
        assert_eq!(status_label(999), "Unknown");
    }
}
