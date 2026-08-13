//! Publieur d'evenements sur un stream Redis.
//!
//! Format du payload identique a sentinel (`{"event": ..., "data": ...}`) pour
//! que le consommateur du bot reste le meme code : un champ `payload` par
//! entry, stream borne par MAXLEN approximatif.

use async_trait::async_trait;
use platform_core::nexus::ports::outbound::events::EventPublisher;

/// Cle du stream Redis des evenements Nexus.
pub const STREAM_KEY: &str = "nexus:events";

/// Borne (approximative) du nombre d'entries conservees dans le stream.
pub const STREAM_MAXLEN: usize = 10_000;

/// Nom du champ portant le JSON de l'evenement dans une entry.
pub const PAYLOAD_FIELD: &str = "payload";

pub struct RedisEventPublisher {
    client: redis::Client,
}

impl RedisEventPublisher {
    /// Ouvre un client Redis (la connexion reelle est etablie a la premiere
    /// publication, `redis::Client` est paresseux).
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            client: redis::Client::open(redis_url)?,
        })
    }

    async fn try_publish(&self, event: &str, data: &serde_json::Value) -> redis::RedisResult<()> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let payload = serde_json::json!({ "event": event, "data": data }).to_string();
        redis::cmd("XADD")
            .arg(STREAM_KEY)
            .arg("MAXLEN")
            .arg("~")
            .arg(STREAM_MAXLEN)
            .arg("*")
            .arg(PAYLOAD_FIELD)
            .arg(payload)
            .query_async::<String>(&mut conn)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl EventPublisher for RedisEventPublisher {
    async fn publish(&self, event: &str, data: serde_json::Value) {
        // Best-effort : le cas d'usage metier a deja reussi, un Redis
        // indisponible ne doit pas remonter en erreur HTTP.
        if let Err(e) = self.try_publish(event, &data).await {
            tracing::warn!(error = %e, event, "publication d'evenement impossible");
        }
    }
}
