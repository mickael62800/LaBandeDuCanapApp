use crate::bootstrap::state::OpsState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use redis::AsyncCommands;
use serde_json::json;
use serde_json::Value;

/// Health check complet : vérifie API + PostgreSQL + Redis.
/// Retourne 200 si tout est OK, 503 si un composant est down.
pub async fn health(State(state): State<OpsState>) -> (StatusCode, Json<Value>) {
    let mut status = "ok";
    let mut http_status = StatusCode::OK;

    // ── PostgreSQL check (via le port SystemProbe) ──
    let pg_ok = state.system_probe.database_responding().await;

    // ── Redis check ──
    let redis_ok = match state.redis_client.get_multiplexed_async_connection().await {
        Ok(mut conn) => conn
            .set_ex::<_, _, ()>("health:ping", "pong", 10)
            .await
            .is_ok(),
        Err(_) => false,
    };

    if !pg_ok || !redis_ok {
        status = "degraded";
        http_status = StatusCode::SERVICE_UNAVAILABLE;
    }

    (
        http_status,
        Json(json!({
            "status": status,
            "components": {
                "api": "ok",
                "postgresql": if pg_ok { "ok" } else { "down" },
                "redis": if redis_ok { "ok" } else { "down" },
            }
        })),
    )
}
