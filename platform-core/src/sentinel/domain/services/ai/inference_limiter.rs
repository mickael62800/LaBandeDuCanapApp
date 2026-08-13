use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;

use crate::sentinel::domain::errors::DomainError;
use tokio::sync::Semaphore;
use tokio::sync::SemaphorePermit;

/// Rate limiter pour les appels d'inference ONNX.
/// Combine un semaphore (concurrence max) et un token bucket (debit max/s).
#[derive(Clone)]
pub struct InferenceRateLimiter {
    semaphore: Arc<Semaphore>,
    max_per_sec: u64,
    tokens: Arc<AtomicU64>,
    last_refill: Arc<std::sync::Mutex<Instant>>,
    max_tokens: u64,
}

impl InferenceRateLimiter {
    /// Cree un nouveau rate limiter.
    /// - `max_concurrent` : nombre max d'inferences simultanées
    /// - `max_per_sec` : nombre max d'inferences par seconde (0 = illimité)
    pub fn new(max_concurrent: usize, max_per_sec: u64) -> Self {
        let max_tokens = max_per_sec * 5; // burst 5 secondes
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_per_sec,
            tokens: Arc::new(AtomicU64::new(max_tokens)),
            last_refill: Arc::new(std::sync::Mutex::new(Instant::now())),
            max_tokens,
        }
    }

    /// Tente d'acquérir un permit pour une inference.
    /// Retourne une erreur si le rate limit est dépassé (429-like).
    pub async fn acquire(&self) -> Result<SemaphorePermit<'_>, DomainError> {
        // 1. Token bucket check (débit)
        if self.max_per_sec > 0 {
            self.refill_tokens();

            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return Err(DomainError::RateLimited(
                    "Inference rate limit exceeded — too many requests per second".to_string(),
                ));
            }
            self.tokens.fetch_sub(1, Ordering::Relaxed);
        }

        // 2. Semaphore check (concurrence)
        match self.semaphore.try_acquire() {
            Ok(permit) => Ok(permit),
            Err(_) => {
                // Toutes les sessions sont occupées, attendre brièvement
                match tokio::time::timeout(
                    tokio::time::Duration::from_secs(5),
                    self.semaphore.acquire(),
                )
                .await
                {
                    Ok(Ok(permit)) => Ok(permit),
                    _ => Err(DomainError::RateLimited(
                        "Inference rate limit exceeded — max concurrent inferences reached"
                            .to_string(),
                    )),
                }
            }
        }
    }

    fn refill_tokens(&self) {
        let mut last = self.last_refill.lock().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        let refill = (elapsed * self.max_per_sec as f64) as u64;

        if refill > 0 {
            let current = self.tokens.load(Ordering::Relaxed);
            let new_val = (current + refill).min(self.max_tokens);
            self.tokens.store(new_val, Ordering::Relaxed);
            *last = now;
        }
    }
}

#[cfg(test)]
#[path = "tests/inference_limiter.rs"]
mod tests;
