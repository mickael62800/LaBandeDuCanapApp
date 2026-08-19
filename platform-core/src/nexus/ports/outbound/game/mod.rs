//! Ports sortants Game Portal — abstractions infrastructure.
//!
//! Domain et application dependent uniquement de ces traits. Les
//! implementations concretes (postgres, docker via bollard, RCON) sont
//! sous adapters/outbound/.

pub mod alert_repository;
pub mod container_runtime;
pub mod game_audit_repository;
pub mod game_server_config_repository;
pub mod game_server_repository;
pub mod game_session_repository;
pub mod game_template_repository;
pub mod player_session_repository;
pub mod port_allocator;
pub mod rcon_client;
pub mod schedule_repository;
