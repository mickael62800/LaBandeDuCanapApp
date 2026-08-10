pub mod audit;
pub mod batching;
pub mod deepseek_moderation_service;
pub mod discord_api;
pub mod inference_service;
pub mod job_client;
pub mod nexus_games;
pub mod postgres;
pub mod redis_cache;
// Nomme a plat comme ses voisins : un module `redis` masquerait la crate du
// meme nom pour tout ce sous-arbre.
pub mod redis_presence;
pub mod redis_service_registry;
pub mod system;
pub mod text_tokenizer;
// Adapter OUTBOUND : implémente le port `EventBroadcasterPort` (publication
// Redis pub/sub vers la gateway) — historiquement rangé côté inbound.
pub mod ws;
