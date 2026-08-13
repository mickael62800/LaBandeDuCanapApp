//! Suivi AFK en vocal : la logique vit dans le core hexagonal. Le bot ne fait
//! que la lier à `UserId`.

use serenity::model::id::UserId;

pub type AfkTracker =
    platform_core::sentinel::domain::services::voice::afk_tracker::AfkTracker<UserId>;
