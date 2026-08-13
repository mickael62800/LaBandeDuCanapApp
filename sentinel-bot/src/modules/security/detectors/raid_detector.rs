//! Détecteur de raid : la logique (fenêtre glissante) vit dans le core
//! hexagonal. Le bot ne fait que la lier à son type d'identifiant Discord.

use serenity::model::id::GuildId;

pub type RaidDetector =
    platform_core::sentinel::domain::services::security::raid_detector::RaidDetector<GuildId>;
