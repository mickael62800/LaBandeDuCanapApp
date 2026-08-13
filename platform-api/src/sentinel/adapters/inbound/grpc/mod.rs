//! Adaptateur entrant gRPC (Phase 7A).
//!
//! Coexiste avec l'adaptateur HTTP/Axum : meme `AppState`, memes use-cases
//! domain. Le serveur tonic ecoute sur un port distinct (`GRPC_PORT`,
//! defaut 50051) et est demarre en parallele depuis `main.rs` via
//! `tokio::spawn`.
//!
//! Conversion d'erreurs : `DomainError` -> `tonic::Status` (cf. `errors`).

pub mod ai;
pub mod audit;
pub mod community;
pub mod errors;
pub mod guild_backup;
pub mod moderation;
pub mod server;
pub mod system;

/// Parse un UUID depuis une string proto. Retourne `Status::invalid_argument` si invalide.
pub(crate) fn parse_uuid(s: &str) -> Result<uuid::Uuid, tonic::Status> {
    uuid::Uuid::from_str(s)
        .map_err(|_| tonic::Status::invalid_argument(format!("UUID invalide: {s}")))
}

use std::str::FromStr;
