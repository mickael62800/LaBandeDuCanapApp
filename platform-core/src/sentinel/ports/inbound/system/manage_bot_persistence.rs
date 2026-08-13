//! Port inbound : persistance des donnees fire-and-forget des bots
//! (streaks de progression, etc.). Le handler HTTP ne fait que
//! parser/RBAC/mapper ; le SQL vit dans `BotPersistenceRepository`.

use async_trait::async_trait;

use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait ManageBotPersistenceUseCase: Send + Sync {
    /// Met a jour les compteurs de streak d'un membre (progression bot).
    /// Idempotent : ecrase les valeurs courantes.
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
