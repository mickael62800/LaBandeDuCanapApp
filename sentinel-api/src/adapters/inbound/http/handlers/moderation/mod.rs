pub mod actions;
pub mod automod;
pub mod infractions;
pub mod purge;
pub mod rules;
pub mod target_risk;

// Glob re-export du fichier `actions.rs` (l'ancien `moderation.rs` au root)
// pour preserver `handlers::moderation::log_action` & co. via leur ancien path.

pub mod sursis;
