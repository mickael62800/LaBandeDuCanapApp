//! Port broadcast d'evenements applicatifs (typiquement vers WebSocket via
//! Redis stream). L'application emet des events; l'adapter route.

#[derive(Debug, Clone, serde::Serialize)]
pub struct WsEvent {
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<String>,
    pub data: serde_json::Value,
}

pub trait EventBroadcaster: Send + Sync {
    fn broadcast(&self, event: &str, data: serde_json::Value);
}
