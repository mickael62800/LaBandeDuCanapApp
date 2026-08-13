use async_trait::async_trait;

use crate::sentinel::domain::entities::moderation::action::strikes::StrikeConfig;
use crate::sentinel::domain::entities::moderation::action::strikes::UserStrike;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait StrikeRepository: Send + Sync {
    async fn save_strike(&self, strike: &UserStrike) -> Result<(), DomainError>;
    async fn find_active_strikes(
        &self,
        guild_id: &str,
        user_id: &str,
        window_secs: i64,
    ) -> Result<Vec<UserStrike>, DomainError>;
    async fn delete_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn delete_strike_by_infraction_id(
        &self,
        infraction_id: uuid::Uuid,
    ) -> Result<u64, DomainError>;
    async fn get_config(&self, guild_id: &str) -> Result<Option<StrikeConfig>, DomainError>;
    async fn save_config(&self, config: &StrikeConfig) -> Result<(), DomainError>;
}
