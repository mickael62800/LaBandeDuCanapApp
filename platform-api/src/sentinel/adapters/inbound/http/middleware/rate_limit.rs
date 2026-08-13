//! Rate limit par IP — re-export depuis `platform-common-api`.
//!
//! L'implementation (token bucket, lecture anti-spoofing de `X-Forwarded-For`,
//! purge des buckets) est partagee avec `nexus-api`. Ce module ne survit que
//! pour preserver le chemin d'import historique.

pub use platform_common_api::rate_limit::rate_limit_middleware;
pub use platform_common_api::rate_limit::RateLimiter;
