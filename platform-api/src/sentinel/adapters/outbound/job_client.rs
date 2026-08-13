use redis::AsyncCommands;
use serde::Serialize;
use tracing::debug;
use tracing::error;
/// Client pour enqueue des jobs dans la queue Redis du worker
#[derive(Clone)]
pub struct JobClient {
    redis: redis::Client,
    queue_key: String,
}

#[derive(Serialize)]
struct Job<'a> {
    #[serde(rename = "type")]
    job_type: &'a str,
    payload: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl JobClient {
    pub fn new(redis: redis::Client, queue_key: String) -> Self {
        Self { redis, queue_key }
    }

    /// Enqueue un job pour le worker
    pub async fn enqueue(&self, job_type: &str, payload: serde_json::Value) {
        let job = Job {
            job_type,
            payload,
            created_at: chrono::Utc::now(),
        };

        let json = match serde_json::to_string(&job) {
            Ok(j) => j,
            Err(e) => {
                error!(error = %e, "Serialization job échouée");
                return;
            }
        };

        let job_type_owned = job_type.to_string();
        let redis = self.redis.clone();
        let key = self.queue_key.clone();

        // Fire-and-forget dans un spawn pour ne pas bloquer le handler
        tokio::spawn(async move {
            match redis.get_multiplexed_async_connection().await {
                Ok(mut conn) => {
                    if let Err(e) = conn.lpush::<_, _, ()>(&key, &json).await {
                        error!(error = %e, "LPUSH job échoué");
                    } else {
                        debug!(job_type = %job_type_owned, "Job enqueued");
                    }
                }
                Err(e) => error!(error = %e, "Redis connection pour job"),
            }
        });
    }
}
