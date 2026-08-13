use async_trait::async_trait;

use crate::sentinel::domain::entities::community::level::UserLevel;
use crate::sentinel::domain::entities::community::level::XpSource;
use crate::sentinel::domain::entities::community::progression_calc::StreakState;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait LevelRepository: Send + Sync {
    /// Lit l'etat de streak persiste (jours consecutifs) d'un utilisateur.
    /// `None` si l'utilisateur n'a pas encore de ligne de progression.
    async fn get_streak(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<StreakState>, DomainError>;
    /// Met a jour l'etat de streak persiste d'un utilisateur.
    async fn update_streak(
        &self,
        guild_id: &str,
        user_id: &str,
        state: StreakState,
    ) -> Result<(), DomainError>;
    async fn get_user_level(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserLevel>, DomainError>;
    async fn upsert_user_level(&self, user: &UserLevel) -> Result<(), DomainError>;
    /// Ajoute de l'XP de maniere atomique (pas de race condition).
    /// Retourne le user_level mis a jour.
    async fn add_xp_atomic(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        amount: i64,
        source: XpSource,
    ) -> Result<UserLevel, DomainError>;
    async fn get_leaderboard(
        &self,
        guild_id: &str,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError>;
    async fn get_leaderboard_by_source(
        &self,
        guild_id: &str,
        source: XpSource,
        limit: i64,
    ) -> Result<Vec<UserLevel>, DomainError>;
    /// Force le refresh de la vue materialized mv_level_leaderboard
    /// (utilise apres une mutation admin set/reset XP pour que le
    /// leaderboard cote frontend voit la valeur a jour immediatement).
    async fn refresh_leaderboard_view(&self) -> Result<(), DomainError>;
}
