//! Cas d'usage de Sentinel.
//!
//! Les services coordonnent les ports entrants et sortants. Un handler HTTP,
//! une commande Discord et un job planifie doivent appeler le meme service
//! lorsqu'ils executent la meme action, afin de partager validation, acces,
//! audit et invalidation de cache.
//!
//! Les services ne connaissent pas Axum ni Serenity. Ils recoivent des valeurs
//! metier et retournent un resultat exploitable par l'adaptateur.

// Helpers transverses.
pub mod validation;

// Bounded contexts.
pub mod ai;
pub mod audit;
pub mod community;
pub mod guild_backup;
pub mod moderation;
pub mod system;
