//! Adapters Game Portal — implementations infra des ports outbound game.
//!
//! - `http_runtime` : impl ContainerRuntime via `docker-agent` (HTTP). Ce
//!   crate ne parle plus au socket Docker et ne depend plus de bollard : le
//!   mapping bollard -> domaine vit dans `docker-agent/src/bollard_game.rs`.
//! - `rcon_minecraft` : impl RconClient via crate `rcon`.
//! - `redis_port_allocator` : impl PortAllocator via Redis SETNX.

pub mod http_runtime;
pub mod noop_runtime;
pub mod rcon_minecraft;
pub mod rcon_pooled;
pub mod redis_port_allocator;
