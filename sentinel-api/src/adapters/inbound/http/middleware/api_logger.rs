use std::sync::Arc;
use std::time::Instant;

use axum::extract::Request;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::Response;

use tracing::warn;

use crate::adapters::inbound::http::state::AppState;
use crate::adapters::outbound::system::rate_limiter::RateLimiter;
use sentinel_core::domain::entities::ops::log_entry::LogEntry;
use sentinel_core::ports::outbound::ops::log_repository::LogRepository;

#[derive(Clone)]
pub struct ApiLoggerState {
    pub log_repo: Arc<dyn LogRepository>,
    pub rate_limiter: Option<Arc<RateLimiter>>,
}

impl ApiLoggerState {
    pub fn from_app(state: &AppState) -> Self {
        Self {
            log_repo: state.log_repo.clone(),
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
    let rate_limiter = s.rate_limiter.clone();
    let method = request.method().to_string();
    let uri = request.uri().path().to_string();
    // Extrait l'IP client : derriere nginx, X-Forwarded-For est l'autoritative.
    // Sinon X-Real-IP ou peer addr (cas dev direct).
    let client_ip = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            request
                .headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "-".to_string());

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
            if let Err(e) = repo.save(&entry).await {
                warn!(error = %e, "Echec sauvegarde log API");
            }
        });
    }

    response
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
