//! Coeur metier de la plateforme.
//!
//! Chaque module de premier niveau est un contexte metier autonome. Les
//! adaptateurs HTTP, Discord, PostgreSQL ou Docker vivent dans d'autres crates.

pub mod atrium;
pub mod nexus;
pub mod ops;
pub mod sentinel;
pub mod shared;
