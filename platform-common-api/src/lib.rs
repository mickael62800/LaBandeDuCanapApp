//! Middlewares HTTP partages par `sentinel-api` et `nexus-api`.
//!
//! Crate separe de `platform-common` pour une raison de dependances : ce qui
//! est ici tire axum, tower-http et le stack metrics. Un bot Discord n'a aucune
//! raison de les compiler.
//!
//! # Contenu
//!
//! - [`rate_limit`] : token bucket par IP, avec lecture anti-spoofing de
//!   `X-Forwarded-For`.
//! - [`metrics`] : recorder Prometheus, middleware de comptage, sampler tokio.
//! - [`http`] : CORS et en-tetes de securite.
//!
//! # Ce qui reste propre a chaque API
//!
//! L'authentification, le verrou mono-serveur et le mapping des erreurs
//! dependent de l'etat applicatif et des regles metier de chaque plateforme :
//! les mutualiser demanderait d'abstraire ce qui les differencie, pour un gain
//! nul.

pub mod http;
pub mod metrics;
pub mod rate_limit;
pub mod errors;

pub use rate_limit::rate_limit_middleware;
pub use rate_limit::RateLimiter;
