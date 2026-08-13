use async_trait::async_trait;

use crate::sentinel::domain::entities::audit::watched_user::WatchedUser;
use crate::sentinel::domain::errors::DomainError;

#[async_trait]
pub trait WatchedUserRepository: Send + Sync {
    async fn find_watched_users(
        &self,
        guild_id: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<WatchedUser>, DomainError>;

    async fn add_manual_watch(
        &self,
        guild_id: &str,
        user_id: &str,
        username: &str,
        reason: &str,
        added_by: &str,
    ) -> Result<(), DomainError>;

    async fn remove_manual_watch(&self, guild_id: &str, user_id: &str) -> Result<(), DomainError>;
}
