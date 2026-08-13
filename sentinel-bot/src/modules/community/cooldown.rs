//! Rate limiter per-user d'interactions : la logique (check-and-set atomique
//! + cleanup inline) vit dans le core hexagonal.

pub use platform_core::sentinel::domain::services::community::interaction_cooldown::InteractionCooldown;
