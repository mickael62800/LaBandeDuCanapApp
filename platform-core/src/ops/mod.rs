//! Contexte Ops : supervision, securite et alertes.
//!
//! Seul le metier appartient au coeur. Les acces privilegies a l'hote restent
//! dans `ops-agent` et `docker-agent`.

pub mod application;
pub mod domain;
pub mod ports;
