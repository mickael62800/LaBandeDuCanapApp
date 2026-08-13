use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::user_stats::UserStats;
use crate::sentinel::domain::entities::audit::user_stats::VoiceSessionStats;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn upsert(&self, stats: &UserStats) -> Result<(), DomainError>;
    async fn find_by_user(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<Option<UserStats>, DomainError>;
    async fn find_by_guild(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<UserStats>, DomainError>;
    async fn increment_messages(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        count: u64,
    ) -> Result<(), DomainError>;
    async fn add_voice_seconds(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        seconds: u64,
    ) -> Result<(), DomainError>;
    async fn count_distinct_guilds(&self) -> Result<u64, DomainError>;
    async fn count_distinct_users(&self) -> Result<u64, DomainError>;
    async fn save_voice_session(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        channel_id: &str,
        channel_name: &str,
        duration_secs: u64,
    ) -> Result<(), DomainError>;
    async fn get_guild_voice_stats(
        &self,
        guild_id: &str,
        days: u32,
        limit: u32,
    ) -> Result<Vec<VoiceSessionStats>, DomainError>;
    async fn count_unique_voice_users(&self, guild_id: &str, days: u32)
        -> Result<i64, DomainError>;
}
