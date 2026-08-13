//! Handlers HTTP du module Automod (Phase 4).
//!
//! Pas de logique metier ici — les handlers reutilisent les use cases
//! (ports inbound). Le module est decoupe en sous-modules cohesifs ;
//! les handlers restent reachable a leur path historique
//! (`handlers::moderation::automod::HANDLER`) via les re-exports ci-dessous.

mod adaptive_slowmode;
mod discussions;
mod dto;
mod fp_stats;
pub(crate) mod reviews;

pub use adaptive_slowmode::*;
pub use discussions::*;
pub use dto::*;
pub use fp_stats::*;
pub use reviews::*;
