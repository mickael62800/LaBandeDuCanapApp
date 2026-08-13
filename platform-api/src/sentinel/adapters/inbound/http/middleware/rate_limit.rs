//! Rate limit par IP — re-export depuis `platform-api::shared`.
//!
//! L'implementation (token bucket, lecture anti-spoofing de `X-Forwarded-For`,
//! purge des buckets) est partagee avec `nexus-api`. Ce module ne survit que
//! pour preserver le chemin d'import historique.

pub use crate::shared::rate_limit::rate_limit_middleware;
pub use crate::shared::rate_limit::RateLimiter;
