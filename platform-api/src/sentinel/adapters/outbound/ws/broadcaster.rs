#[cfg(test)]
#[path = "tests/broadcaster.rs"]
mod tests;

use tracing::warn;

pub use platform_core::sentinel::ports::outbound::system::event_broadcaster::{
    EventBroadcaster as EventBroadcasterPort, WsEvent,
};

/// Nom de la stream Redis partagee par tous les producers.
/// Phase 5B : doit rester synchronise avec `sentinel-bot/src/shared/event_bus.rs` (STREAM_KEY).
const STREAM_KEY: &str = "sentinel:events";
/// Borne de taille approximative de la stream (voir event_bus::STREAM_MAXLEN).
const STREAM_MAXLEN: usize = 10_000;
const PAYLOAD_FIELD: &str = "payload";

/// Broadcaster d'evenements — XADD sur la stream `sentinel:events`.
/// Gateway lit en live-tail (XREAD $) et relay vers les WebSockets desktop.
pub struct EventBroadcaster {
    redis_client: Option<redis::Client>,
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBroadcaster {
    pub fn new() -> Self {
        Self { redis_client: None }
    }

    /// Configure la publication Redis.
    pub fn with_redis(mut self, client: redis::Client, _channel: String) -> Self {
        // `_channel` garde pour compat API historique — remplace par STREAM_KEY.
        self.redis_client = Some(client);
        self
    }

    /// Publie un evenement sur la stream Redis.
    /// Le `guild_id` est extrait automatiquement du payload JSON pour le filtrage server-side.
    pub fn broadcast(&self, event: &str, data: serde_json::Value) {
        let guild_id = data
            .get("guild_id")
            .and_then(|v| v.as_str())
            .map(String::from);

        let ws_event = WsEvent {
            event: event.to_string(),
            guild_id,
            data,
        };

        if let Some(ref client) = self.redis_client {
            let client = client.clone();
            let json = match serde_json::to_string(&ws_event) {
                Ok(j) => j,
                Err(e) => {
                    warn!(error = %e, event = %ws_event.event, "Echec serialisation event broadcast — event perdu");
                    return;
                }
            };
            tokio::spawn(async move {
                match client.get_multiplexed_async_connection().await {
                    Ok(mut conn) => {
                        let res: redis::RedisResult<String> = redis::cmd("XADD")
                            .arg(STREAM_KEY)
                            .arg("MAXLEN")
                            .arg("~")
                            .arg(STREAM_MAXLEN)
                            .arg("*")
                            .arg(PAYLOAD_FIELD)
                            .arg(&json)
                            .query_async(&mut conn)
                            .await;
                        if let Err(e) = res {
                            warn!(error = %e, "Echec Redis XADD event broadcast");
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "Echec connexion Redis pour broadcast");
                    }
                }
            });
        }
    }
}

impl EventBroadcasterPort for EventBroadcaster {
    fn broadcast(&self, event: &str, data: serde_json::Value) {
        EventBroadcaster::broadcast(self, event, data)
    }
}
