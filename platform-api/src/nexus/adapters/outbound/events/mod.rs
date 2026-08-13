//! Adapters du port `EventPublisher` : publication des evenements Nexus vers
//! le bot Discord.
//!
//! Deux implementations :
//!   - `RedisEventPublisher` : stream Redis `nexus:events` (production) ;
//!   - `NoopEventPublisher`  : ne publie rien (dev sans Redis).

pub mod noop_publisher;
pub mod redis_publisher;
