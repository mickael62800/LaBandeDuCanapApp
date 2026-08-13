use async_trait::async_trait;
use uuid::Uuid;

use crate::sentinel::domain::entities::moderation::action::strikes::StrikeConfig;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeResult;
use crate::sentinel::domain::entities::moderation::action::strikes::StrikeThreshold;
use crate::sentinel::domain::entities::moderation::action::strikes::UserStrike;
use crate::sentinel::domain::entities::system::discord_ids::GuildId;
use crate::sentinel::domain::entities::system::discord_ids::UserId;
use crate::sentinel::domain::errors::DomainError;

pub struct AddStrikeCommand {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub reason: String,
    pub source: String,
    pub infraction_id: Option<Uuid>,
}

pub struct SaveStrikeConfigCommand {
    pub guild_id: GuildId,
    pub window_secs: i64,
    pub thresholds: Vec<StrikeThreshold>,
    pub enabled: bool,
}

#[async_trait]
pub trait ManageStrikesUseCase: Send + Sync {
    async fn add_strike(&self, cmd: AddStrikeCommand) -> Result<StrikeResult, DomainError>;
    async fn get_active_strikes(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Vec<UserStrike>, DomainError>;
    async fn reset_strikes(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
    async fn get_config(&self, guild_id: &str) -> Result<StrikeConfig, DomainError>;
    async fn save_config(&self, cmd: SaveStrikeConfigCommand) -> Result<StrikeConfig, DomainError>;
}
