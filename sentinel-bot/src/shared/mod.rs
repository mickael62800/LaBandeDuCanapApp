pub mod api_client;
pub mod cache_settings;
pub mod circuit_breaker;
pub mod config;
pub mod discord_helpers;
pub mod embeds;
pub mod event_bus;
pub mod event_signing;
/// Signature des events publies VERS une autre plateforme (Atrium). Secret
/// distinct de `SENTINEL_API_KEY` — cf. l'en-tete du module.
pub mod platform_event_signing;
pub mod grpc_client;
pub mod heartbeat;
pub mod parsers;
pub mod presence;
pub mod shard_launcher;
pub mod svg;
