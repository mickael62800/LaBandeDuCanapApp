//! Détection de flood : la logique (fenêtre glissante + seuils à chaud) vit
//! dans le core hexagonal. Le bot ne fait que la lier à ses types Discord.

use serenity::model::id::{ChannelId, UserId};

pub type FloodTracker =
    platform_core::sentinel::domain::services::voice::flood_tracker::FloodTracker<
        ChannelId,
        UserId,
    >;
