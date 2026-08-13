//! Tracker de progression : la logique (agrégation activité + anti-farm XP AFK)
//! vit dans le core hexagonal, réexportée ici. Le bot ne fait que l'alimenter
//! avec les événements Discord.

pub use platform_core::sentinel::domain::services::progression::tracker::*;
