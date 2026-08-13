//! Port outbound : persistance des donnees fire-and-forget des bots
//! (`user_levels.streak_*`, etc.). Tout le SQL vit dans l'adapter Postgres.

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait BotPersistenceRepository: Send + Sync {
    /// UPDATE des compteurs de streak d'un membre dans `user_levels`.
    async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        streak_current: i32,
        streak_best: i32,
        streak_last_day: i32,
        streak_last_year: i32,
    ) -> Result<(), DomainError>;
}
