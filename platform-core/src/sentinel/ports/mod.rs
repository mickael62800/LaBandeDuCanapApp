//! Contrats de l'architecture hexagonale.
//!
//! `inbound` expose les capacites demandees par l'API, le bot et les workers.
//! `outbound` decrit les dependances consommees par le core (PostgreSQL,
//! Redis, Discord, IA). Les implementations vivent dans les adaptateurs.

pub mod inbound;
pub mod outbound;
pub mod uow;
