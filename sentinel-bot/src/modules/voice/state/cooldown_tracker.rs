//! Cooldown par utilisateur : la logique (check-and-set atomique anti-TOCTOU)
//! vit dans le core hexagonal. Le bot ne fait que la lier à `UserId`.

use serenity::model::id::UserId;

pub type CooldownTracker =
    platform_core::sentinel::domain::services::voice::cooldown_tracker::CooldownTracker<UserId>;
