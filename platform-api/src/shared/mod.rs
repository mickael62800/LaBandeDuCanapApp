//! Infrastructure HTTP partagée par les quatre domaines de `platform-api`.
//!
//! Ce module reste séparé de `platform-common` : il tire Axum, Tower HTTP et
//! la pile de métriques, dont les bots n'ont pas besoin.
//!
//! # Contenu
//!
//! - [`rate_limit`] : token bucket par IP, avec lecture anti-spoofing de
//!   `X-Forwarded-For`.
//! - [`metrics`] : recorder Prometheus, middleware de comptage, sampler tokio.
//! - [`http`] : CORS et en-tetes de securite.
//! - [`auth_client`] : client de `auth-api`, pour les APIs qui doivent savoir
//!   QUI appelle. Il ne porte aucune regle — la decision appartient a
//!   l'identite.
//!
//! # Ce qui reste propre a chaque API
//!
//! Le verrou mono-serveur et le mapping des erreurs dependent de l'etat
//! applicatif et des regles metier de chaque plateforme : les mutualiser
//! demanderait d'abstraire ce qui les differencie, pour un gain nul.
//!
//! L'authentification, elle, n'est plus « propre a chaque API » : elle est
//! servie par `auth-api` et consommee ici de la meme facon par tout le monde.

pub mod auth_client;
pub mod bearer_auth;
pub mod errors;
pub mod http;
pub mod job_lock;
pub mod metrics;
pub mod rate_limit;

pub use rate_limit::rate_limit_middleware;
pub use rate_limit::RateLimiter;
