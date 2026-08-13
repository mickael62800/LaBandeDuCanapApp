use std::sync::Arc;

use axum::extract::State;
use axum::Json;

use crate::broadcaster::EventBroadcaster;

pub async fn health(State(broadcaster): State<Arc<EventBroadcaster>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "platform-gateway",
        "connected_clients": broadcaster.connected_count(),
    }))
}
