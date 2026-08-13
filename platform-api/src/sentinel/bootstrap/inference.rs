//! Inference ONNX (vision + text + rate limiter) et EventBroadcaster Redis.

use std::sync::Arc;

use crate::sentinel::adapters::outbound::inference_service::InferenceService;
use crate::sentinel::adapters::outbound::text_tokenizer::TextTokenizer;
use crate::sentinel::adapters::outbound::ws::broadcaster::EventBroadcaster;
use platform_core::sentinel::domain::services::ai::inference_limiter::InferenceRateLimiter;
use tracing::info;

/// Construit le service d'inference ONNX (vision + text tokenizer + rate limiter).
pub fn build_inference() -> (
    Arc<InferenceService>,
    Arc<TextTokenizer>,
    Arc<InferenceRateLimiter>,
) {
    let vision_model_path = std::env::var("VISION_MODEL_PATH").ok();
    let text_model_path = std::env::var("TEXT_MODEL_PATH").ok();
    let tokenizer_path = std::env::var("TEXT_TOKENIZER_PATH").ok();
    let text_max_length: usize = std::env::var("TEXT_MAX_LENGTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(512);

    let inference = Arc::new(InferenceService::new(
        vision_model_path.as_deref(),
        text_model_path.as_deref(),
    ));
    let tokenizer = Arc::new(TextTokenizer::new(
        tokenizer_path.as_deref(),
        text_max_length,
    ));

    let inference_max_concurrent: usize = std::env::var("INFERENCE_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4);
    let inference_max_per_sec: u64 = std::env::var("INFERENCE_MAX_PER_SEC")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let inference_limiter = Arc::new(InferenceRateLimiter::new(
        inference_max_concurrent,
        inference_max_per_sec,
    ));

    info!(
        max_concurrent = inference_max_concurrent,
        max_per_sec = inference_max_per_sec,
        "Inference rate limiter configure"
    );

    (inference, tokenizer, inference_limiter)
}

/// Construit l'EventBroadcaster connecte a Redis pub/sub.
pub fn build_broadcaster(redis_client: redis::Client) -> Arc<EventBroadcaster> {
    let redis_channel =
        std::env::var("REDIS_CHANNEL").unwrap_or_else(|_| "sentinel:events".to_string());
    Arc::new(EventBroadcaster::new().with_redis(redis_client, redis_channel))
}
