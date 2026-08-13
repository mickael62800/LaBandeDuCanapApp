//! # nexus-core — coeur hexagonal de la plateforme jeux Nexus
//!
//! Bibliothèque métier pure de NEXUS : domaine, cas d'usage et ports.
//!
//! ## Regles d'architecture (identiques a sentinel-core)
//! - AUCUNE dependance infra : pas de `sqlx`, `axum`, `reqwest`, `redis`,
//!   ni `serenity`. Seules les deps "pures" sont admises (serde, thiserror,
//!   chrono, uuid).
//! - `domain` n'importe NI `ports` NI `application` : entites, services de
//!   domaine et enums purs uniquement.
//! - `application` orchestre le domaine via les `ports` (traits).
//! - `ports::inbound` = cas d'usage exposes ; `ports::outbound` = besoins
//!   d'infra abstraits (repos, gateways), implementes par les adapters des
//!   binaires (`nexus-api`, `nexus-bot`, `nexus-worker`, `nexus-gateway`).
//!
//! NEXUS regroupe le portail de serveurs de jeu, les wallets, la Roue du
//! Destin, le Coussin Piégé, les jeux mentionnables et le Grand Salon.

pub mod application;
pub mod domain;
pub mod ports;
