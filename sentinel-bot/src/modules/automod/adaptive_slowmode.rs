//! Slowmode adaptatif : la logique (fenêtre glissante + décision) vit dans le
//! core hexagonal. Le bot ne fait que la lier à son type de salon Discord.

use serenity::model::id::ChannelId;

pub type SlowmodeTracker =
    platform_core::sentinel::domain::services::automod::adaptive_slowmode::SlowmodeTracker<
        ChannelId,
    >;
